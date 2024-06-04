use clap::{ArgAction, Parser, Subcommand};
use std::{
    fs::{create_dir, remove_dir_all, remove_file, File},
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    path::Path,
    process::{exit, Command},
    time::Duration,
};

use helpers::{
    chmod, command_output_formater, daemon_running, feddit_archivieren_assert, get,
    print_to_update_log, read_from_stream, read_pid_file, root, run_command, update,
};

mod helpers;
mod settings;

#[derive(Subcommand)]
enum Commands {
    /// Startet den Daemon
    Start,
    /// Killt den Daemon (ohne zu Daten zu sichern)
    Kill,
    /// Updated das Programm auf die neuste Version
    Update,
    /// Löscht alle Dateien vom Programm, bis auf die binarys
    Clean,
    /// Zeigt Informationen über den Daemon an
    Info,
    /// Überprüft den Gesundheitszustand des Daemons
    Checkhealth,
    /// Stoppt den Daemon (sichere Version von kill)
    Stop,
    /// Printet Live was der Daemon ausgibt
    Listen,
    /// Deinstalliert das Programm (ruft auch Clean)
    Uninstall,
    /// (DEBUG) Installiert das Programm
    Install,
    /// (DEBUG) Updated das Programm mit den Dateien im aktuellen Verzeichnis
    UpdateLocal,
    /// (DEBUG) Killt den Daemon wenn er läuft
    KillMaybe,
    /// (DEBUG) Zeigt die Logs des Daemons an
    LogsStatic,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    subcommand: Commands,

    /// Erzwingt die gegebene Aktion (genaues Verhalten variiert)
    #[arg(short, long, action = ArgAction::SetTrue, global = true)]
    force: bool,

    /// Gibt bei "install" an, ob es sich um einen debug build handelt oder nicht, d.h. ob sich die
    /// binarys in target/debug oder target/release befinden.
    #[arg(short, long, action = ArgAction::SetTrue, global = true)]
    dev_build: bool,
}

fn main() {
    let args = Cli::parse();
    let force = args.force;
    let dev_build = args.dev_build;

    match args.subcommand {
        Commands::Install => {
            let mut replace_daemon = false;
            if daemon_running() {
                if force {
                    println!("Force-Kille den Daemon...");
                    kill_daemon();
                } else {
                    print_formatted_to_update_log!("Es laeuft bereits ein Daemon, versuche ihn zu restarten mit der neuen Version...");
                    replace_daemon = true;
                    if let Err(err) = restart_daemon() {
                        print_formatted_to_update_log!("Fehler beim Stoppen des Daemons: {}", err);
                        exit(1);
                    }
                    print_formatted_to_update_log!("Gestoppt!");
                }
            }

            // Die alten Binarys löschen
            remove_if_existing(settings::DAEMON_PATH);
            remove_if_existing(settings::CLIENT_PATH);

            // Die neuen an die richtige Stelle kopieren
            if dev_build {
                copy_file("target/debug/daemon", settings::DAEMON_PATH);
                copy_file("target/debug/client", settings::CLIENT_PATH);
            } else {
                copy_file("target/release/daemon", settings::DAEMON_PATH);
                copy_file("target/release/client", settings::CLIENT_PATH);
            }

            // Das Update und Run-Verzeichnis erstellen
            create_run_dir();

            if !Path::new(settings::UDPATE_DIR).exists() {
                if let Err(err) = create_dir(settings::UDPATE_DIR) {
                    let msg = &format!(
                        "Fehler beim Erstellen von {}: {}",
                        settings::UDPATE_DIR,
                        err
                    );
                    if replace_daemon {
                        print_formatted_to_update_log!("{}", msg);
                    } else {
                        eprintln!("{}", msg);
                    };
                }
            }

            // Jedem Benutzer read-write-execute Rechte für die Dateien geben, wenn möglich
            if root() {
                chmod(settings::DAEMON_PATH, "777");
                chmod(settings::CLIENT_PATH, "777");
            }

            println!("Installation erfolgreich!");

            if replace_daemon {
                print_formatted_to_update_log!("Starte den Daemon neu...");
                start_daemon!(print_to_update_log);
            }
        }
        Commands::Start => {
            if daemon_running() {
                if force {
                    kill_daemon();
                } else {
                    eprintln!("Der Daemon läuft bereits.");
                    exit(1);
                }
            }

            start_daemon!();
        }
        Commands::Kill => {
            kill_daemon();
        }
        Commands::KillMaybe => {
            if daemon_running() {
                kill_daemon();
            }
        }
        Commands::Update => {
            if force && daemon_running() {
                kill_daemon();
            } else {
                feddit_archivieren_assert(!daemon_running(), "Der Daemon läuft gerade.");
                feddit_archivieren_assert(root(), "Du must root sein.");
            }

            // Die Update Funktion rufen, auf das Ergebnis reagieren
            if let Err(message) = update(None, None) {
                eprintln!("Fehler beim Updaten: ");
                eprintln!("{}", message);
                exit(1);
            }

            exit(0);
        }
        Commands::UpdateLocal => {
            if !force {
                feddit_archivieren_assert(root(), "Du must root sein.");
            }

            if daemon_running() {
                if let Err(err) = stop_daemon() {
                    eprintln!("Fehler beim Stoppen des Daemons: {}", err);
                }
            }

            // `make clean install` ausführen
            println!("Compile den Source Code...");
            match Command::new("make").arg("clean").arg("install").output() {
                Ok(output) => {
                    if !output.status.success() {
                        eprintln!("Fehler bei der Installation.");
                        eprintln!("{}", command_output_formater(&output));
                        exit(1);
                    }
                }
                Err(err) => {
                    eprintln!("Fehler bei der Installation: {}", err);
                    exit(1);
                }
            }
            println!("Lokales Update erfolgreich abgeschlossen.");
            exit(0);
        }
        Commands::Clean => {
            exit(clean());
        }
        Commands::Info => {
            println!("Feddit-Archivieren Version {}", env!("CARGO_PKG_VERSION"));
            if !daemon_running() {
                println!("Der Daemon läuft nicht.")
            } else {
                println!("Der Daemon läuft.");
                println!("Port:\t{}", get(settings::SOCKET_FILE));
                println!("PID:\t{}", get(settings::PID_FILE));
            }
        }
        Commands::LogsStatic => {
            // Printet den Inhalt von OUT_FILE und ERR_FILE

            match File::open(settings::OUT_FILE) {
                Ok(file) => {
                    let mut iterator = BufReader::new(&file).lines().peekable();
                    if iterator.peek().is_some() {
                        println!("STDOUT:");
                        while let Some(line) = iterator.next() {
                            println!(
                                "{}",
                                line.unwrap_or("<FEHLER BEIM LESEN DIESER ZEILE>".into())
                            );
                        }
                    }
                }
                Err(err) => {
                    println!("Fehler beim Lesen von {}: {}", settings::OUT_FILE, err);
                }
            }
            match File::open(settings::ERR_FILE) {
                Ok(file) => {
                    let mut iterator = BufReader::new(&file).lines().peekable();
                    if iterator.peek().is_some() {
                        println!("STDERR:");
                        while let Some(line) = iterator.next() {
                            println!(
                                "{}",
                                line.unwrap_or("<FEHLER BEIM LESEN DIESER ZEILE>".into())
                            );
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Fehler beim Lesen von {}: {}", settings::ERR_FILE, err);
                }
            }
        }
        Commands::Checkhealth => {
            if force && !daemon_running() {
                start_daemon!();
            } else {
                feddit_archivieren_assert(daemon_running(), "Der Daemon läuft nicht.");
            }

            // Schickt `ping` an den Daemon, erwartet `pong`

            println!("Versuche Daten in den Stream zu schreiben.");
            let mut stream = send_to_daemon("ping");
            println!("Fertig.");

            println!("Versuche Daten aus dem Stream zu empfangen.");
            let message = read_from_stream(&mut stream);

            feddit_archivieren_assert(
                message == "pong",
                format!("Nachricht pong erwartet, '{}' empfangen.", message).as_str(),
            );
            println!("Nachricht pong erfolgreich empfangen!");
            println!("Der Daemon scheint zu funktionieren.");
        }
        Commands::Stop => {
            if force && !daemon_running() {
                start_daemon!();
            } else {
                feddit_archivieren_assert(daemon_running(), "Der Daemon läuft nicht.");
            }

            // Sendet `stop` an den Daemon, erwartet`ok`
            if let Err(err) = stop_daemon() {
                eprintln!("Ein Fehler ist aufgetreten:\n{}", err);
                exit(1);
            }

            println!("Der Daemon wurde erfolgreich beendet!");
        }
        Commands::Listen => {
            if force && !daemon_running() {
                start_daemon!();
            } else {
                feddit_archivieren_assert(daemon_running(), "Der Daemon läuft nicht.");
            }

            // Sendet `listen` an den Daemon, printet alles was empfangen wird
            let mut stream = send_to_daemon("listen");
            loop {
                let response = read_from_stream(&mut stream);
                if response.is_empty() {
                    println!("Der Daemon hat die Verbindung geschlossen.");
                    exit(0);
                }
                if response.to_lowercase().trim() == "restart" {
                    println!("Der Daemon wird neu gestartet.");
                    if daemon_running() {
                        if wait_with_timeout!(|| !daemon_running(), Duration::from_millis(500)) {
                            println!("Der Daemon wurde gestoppt.");
                        } else {
                            println!("Der Daemon wurde innerhalb von 0.5 Sekunden nicht beendet.");
                            exit(1);
                        }
                    }

                    if wait_with_timeout!(|| daemon_running(), Duration::from_secs(5)) {
                        println!("Der Daemon ist wieder online!");
                    } else {
                        println!(
                            "Der Daemon ist innerhalb von 5 Sekunden nicht wieder online gegangen."
                        );
                        exit(1);
                    }

                    println!("Stelle Verbindung wieder her...");
                    stream = send_to_daemon("listen");
                } else {
                    println!("{}", response);
                }
            }
        }
        Commands::Uninstall => {
            clean();
            if let Err(err) = remove_file(settings::CLIENT_PATH) {
                eprintln!("Fehler beim Löschen von {}: {}", settings::CLIENT_PATH, err);
            }
            if let Err(err) = remove_file(settings::DAEMON_PATH) {
                eprintln!("Fehler beim Löschen von {}: {}", settings::DAEMON_PATH, err);
            }
        }
    }
}

/// Kopiert eine Datei von from zu to
fn copy_file(from: &str, to: &str) {
    run_command(Command::new("cp").arg(from).arg(to));
}

/// Returnt true wenn das Run-Verzeichnis existiert, false wenn nicht
fn run_dir_exists() -> bool {
    Path::new(settings::RUN_DIR).exists()
}

/// Erstellt das Verzeichnis in das der Daemon seine Logs und Informationen schreibt, wenn es noch
/// nicht existiert
fn create_run_dir() {
    if !run_dir_exists() {
        if let Err(err) = create_dir(settings::RUN_DIR) {
            eprintln!("Fehler beim Erstellen von {}: {}", settings::RUN_DIR, err);
            exit(1);
        }
    }
}

/// Löscht eine Datei, sollte sie existieren
fn remove_if_existing(filepath: &str) {
    if Path::new(filepath).exists() {
        if let Err(err) = remove_file(filepath) {
            eprintln!("Fehler beim Löschen von {}: {}", filepath, err);
            exit(1);
        }
    }
}

/// Killt den Daemon (unsichere Variante von stop)
fn kill_daemon() {
    feddit_archivieren_assert(daemon_running(), "Der Daemon läuft nicht.");
    feddit_archivieren_assert(root(), "Du bist nicht root.");

    if !run_dir_exists() {
        eprintln!("Der Daemon läuft, aber {} existiert nicht, weshalb ich nicht weiß wen ich killen soll.", settings::RUN_DIR);
        eprintln!("Probiers mal mit dem pkill Befehl?");
    }

    match Command::new("kill").arg(read_pid_file()).output() {
        Ok(output) => {
            if !output.status.success() {
                eprintln!("Fehler beim Killen des Daemons:");
                eprintln!("{}", command_output_formater(&output));
            } else {
                println!("Daemon erfolgreich gekillt.");
            }
        }
        Err(err) => {
            eprintln!("Fehler beim Killen des Daemons: {}", err);
        }
    }
}

/// Öffnet einen TcpStream mit dem Daemon und schreibt eine Nachricht hinein.
/// Returnt am Ende den erstellten TcpStream.
fn send_to_daemon(message: &str) -> TcpStream {
    // Den Stream erstellen
    // Das Ziel ist die Adresse die der Daemon ins SOCKET_FILE geschrieben hat
    let mut stream = match TcpStream::connect(get(settings::SOCKET_FILE)) {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!(
                "Fehler beim Verbinden mit {}: {}",
                settings::SOCKET_FILE,
                err
            );
            exit(1);
        }
    };

    // Die Nachricht in den Stream schreiben
    if let Err(err) = stream.write_all(message.as_bytes()) {
        eprintln!("Fehler beim Senden an den Daemon: {}", err);
        exit(1);
    }

    // Den Stream returnen
    stream
}

#[macro_export]
macro_rules! start_daemon {
    () => {{
        _start_daemon(None)
    }};
    ($print:expr) => {{
        _start_daemon(Some($print))
    }};
}

fn _start_daemon(print_override: Option<fn(&str)>) {
    macro_rules! print_maybe_override {
        ($($e:expr), *) => {
            if let Some(override_function) = print_override {
                override_function(&format!($($e), *))
            } else {
                println!($($e), *);
            }
        };
    }

    // Das Run-Verzeichnis für den Daemon erstellen
    create_run_dir();

    // Den Daemon launchen
    match Command::new(settings::DAEMON_PATH).output() {
        Ok(output) => {
            if !output.status.success() {
                print_maybe_override!("Fehler beim Starten des Daemons:");
                print_maybe_override!("{}", command_output_formater(&output));
                exit(1);
            }

            if wait_with_timeout!(|| daemon_running(), Duration::from_secs(1)) {
                print_maybe_override!("Daemon erfolgreich gestartet!");
            } else {
                print_maybe_override!("Der Daemon ist nicht online gegangen.");
            }
        }
        Err(err) => {
            print_maybe_override!("Fehler beim Starten des Daemons: {}", err);
            exit(1);
        }
    }
}

/// Funktion die vom Feddit-Thread ausgeführt wird
#[allow(dead_code)]
fn feddit() {}

/// Löscht RUN_DIR und UPDATE_DIR
fn clean() -> i32 {
    let mut exit_code = 0;

    if daemon_running() {
        kill_daemon();
    }

    if Path::new(settings::RUN_DIR).exists() {
        if let Err(error) = remove_dir_all(settings::RUN_DIR) {
            eprintln!("Fehler beim Löschen von {}: {}", settings::RUN_DIR, error);
            exit_code = 1;
        }
    }
    if Path::new(settings::UDPATE_DIR).exists() {
        if let Err(error) = remove_dir_all(settings::UDPATE_DIR) {
            eprintln!(
                "Fehler beim Löschen von {}: {}",
                settings::UDPATE_DIR,
                error
            );
            exit_code = 1;
        }
    }
    exit_code
}

/// Stoppt den Daemon sicher
fn stop_daemon() -> Result<(), String> {
    let mut stream = send_to_daemon("stop");
    let response = read_from_stream(&mut stream);
    if !(response == "ok") {
        return Err(format!(
            "Der Daemon hat eine unerwartete Antwort gesendet: {}",
            response
        ));
    }

    // Darauf warten, dass der Daemon exitet, maximal 1 Sekunde lang warten
    let daemon_stopped = wait_with_timeout!(daemon_running, Duration::from_secs(1));

    if !daemon_stopped {
        Err("Der Daemon hat eine Bestätigung gesendet, läuft aber immer noch.".to_string())
    } else {
        Ok(())
    }
}

/// Restartet den Daemon
fn restart_daemon() -> Result<(), String> {
    let mut stream = send_to_daemon("restart");
    let response = read_from_stream(&mut stream);
    if !(response == "ok") {
        return Err(format!(
            "Der Daemon hat eine unerwartete Antwort gesendet: {}",
            response
        ));
    }

    // Darauf warten, dass der Daemon exitet, maximal 1 Sekunde lang warten
    let daemon_stopped = wait_with_timeout!(|| !daemon_running(), Duration::from_secs(1));

    if !daemon_stopped {
        Err("Der Daemon hat eine Bestätigung gesendet, läuft aber immer noch.".to_string())
    } else {
        Ok(())
    }
}
