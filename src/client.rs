use std::{
    env::args,
    fs::{create_dir, remove_dir_all, remove_file, File},
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    path::Path,
    process::{exit, Command},
    time::{Duration, Instant},
};

use helpers::{
    chmod, command_output_formater, daemon_running, feddit_archivieren_assert, get,
    read_from_stream, read_pid_file, root, run_install_command,
};

mod helpers;
mod settings;

fn main() {
    let args = args().collect::<Vec<String>>();
    if args.len() <= 1 {
        println!("Kein Befehl angegeben.");
        exit(1);
    }

    match args.get(1).unwrap().as_str() {
        "install" => {
            feddit_archivieren_assert(!daemon_running(), "Es läuft aktuell schon ein Daemon!");

            // Die alten Binarys löschen
            remove_if_existing(settings::DAEMON_PATH);
            remove_if_existing(settings::CLIENT_PATH);

            // Die neuen an die richtige Stelle kopieren
            copy_file("target/debug/daemon", settings::DAEMON_PATH);
            copy_file("target/debug/client", settings::CLIENT_PATH);

            // Jedem Benutzer read-write-execute Rechte für die Dateien geben, wenn möglich
            if root() {
                chmod(settings::DAEMON_PATH, "777");
                chmod(settings::CLIENT_PATH, "777");
            }

            println!("Installation erfolgreich!");
        }
        "start" => {
            feddit_archivieren_assert(!daemon_running(), "Der Daemon läuft bereits.");

            // Das Run-Verzeichnis für den Daemon erstellen
            create_run_dir();

            // Den Daemon launchen
            match Command::new(settings::DAEMON_PATH).output() {
                Ok(output) => {
                    if !output.status.success() {
                        eprintln!("Fehler beim Starten des Daemons:");
                        eprintln!("{}", command_output_formater(&output));
                        exit(1);
                    }

                    println!("Daemon erfolgreich gestartet!");
                    exit(0);
                }
                Err(err) => {
                    eprintln!("Fehler beim Starten des Daemons: {}", err);
                    exit(1);
                }
            }
        }
        "kill" => {
            kill_daemon();
        }
        "kill_maybe" => {
            if daemon_running() {
                kill_daemon();
            }
        }
        "update" => {
            feddit_archivieren_assert(!daemon_running(), "Der Daemon läuft gerade.");
            feddit_archivieren_assert(root(), "Du must root sein.");

            // Die Update Funktion rufen, auf das Ergebnis reagieren
            if let Err(message) = update() {
                eprintln!("Fehler beim Updaten: ");
                eprintln!("{}", message);
                exit(1);
            } else {
                println!("Update erfolgreich abgeschlossen.");
            }

            exit(0);
        }
        "update_local" => {
            feddit_archivieren_assert(root(), "Du must root sein.");

            // TODO: Besser Lösung mit "stop" implementieren
            if daemon_running() {
                kill_daemon();
            }

            // `make clean install` ausführen
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
        "clean" => {
            let mut exit_code = 0;
            // Löscht RUN_DIR und UPDATE_DIR
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
            exit(exit_code);
        }
        "info" => {
            if !daemon_running() {
                println!("Der Daemon läuft nicht.")
            } else {
                println!("Der Daemon läuft.");
                println!("Port:\t{}", get(settings::SOCKET_FILE));
                println!("PID:\t{}", get(settings::PID_FILE));
            }
        }
        "logs_static" => {
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
        "checkhealth" => {
            feddit_archivieren_assert(daemon_running(), "Der Daemon läuft nicht.");

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
        "stop" => {
            feddit_archivieren_assert(daemon_running(), "Der Daemon läuft nicht.");

            // Sendet `stop` an den Daemon, erwartet `ok`

            let mut stream = send_to_daemon("stop");
            let response = read_from_stream(&mut stream);
            feddit_archivieren_assert(
                response == "ok",
                format!(
                    "Der Daemon hat eine unerwartete Antwort gesendet: {}",
                    response
                )
                .as_str(),
            );

            // Darauf warten, dass der Daemon exitet, maximal 1 Sekunde lang warten
            let start = Instant::now();
            while daemon_running() && start.elapsed() < Duration::from_secs(1) {}

            feddit_archivieren_assert(
                !daemon_running(),
                "Der Daemon hat eine Bestätigung gesendet, läuft aber immer noch.",
            );

            println!("Der Daemon wurde erfolgreich beendet!");
        }
        "listen" => {
            feddit_archivieren_assert(daemon_running(), "Der Daemon läuft nicht.");

            // Sendet `listen` an den Daemon, printet alles was empfangen wird
            let mut stream = send_to_daemon("listen");
            loop {
                let response = read_from_stream(&mut stream);
                if response.is_empty() {
                    println!("Der Daemon hat die Verbindung geschlossen.");
                    exit(0);
                }
                println!("{}", response);
            }
        }
        _ => {
            println!("Unbekannter Befehl.");
            exit(1);
        }
    }
}

/// Kopiert eine Datei von from zu to
fn copy_file(from: &str, to: &str) {
    run_install_command(Command::new("cp").arg(from).arg(to));
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

/// Updatet das Programm
fn update() -> Result<(), String> {
    if !Path::new(settings::UDPATE_DIR).exists() {
        // Wenn das Verzeichnis noch nicht existiert, den Code dahinklonen
        match Command::new("git")
            .arg("clone")
            .arg(settings::GITHUB_LINK)
            .arg(settings::UDPATE_DIR)
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    return Err(format!(
                        "Fehler beim Klonen von {} nach {}:\n{}",
                        settings::GITHUB_LINK,
                        settings::UDPATE_DIR,
                        command_output_formater(&output)
                    ));
                }
                println!("{}", command_output_formater(&output));
            }

            Err(err) => {
                return Err(format!(
                    "Fehler beim Klonen von {} nach {}: {}",
                    settings::GITHUB_LINK,
                    settings::UDPATE_DIR,
                    err
                ));
            }
        }
    } else {
        // Das Directory existiert schon, daher pullen wir einfach den neuen Code
        match Command::new("git")
            .current_dir(settings::UDPATE_DIR)
            .arg("pull")
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    let mut message = String::from("Fehler beim Pullen des neuen Codes: ");
                    message.push_str(command_output_formater(&output).as_str());
                    return Err(message);
                }
            }
            Err(message) => {
                return Err(message.to_string());
            }
        }
    }

    println!("Fertig.");
    println!("Compile den Source Code...");

    // Den Code mithilfe des Makefiles compilen und installieren
    match Command::new("make")
        .current_dir(settings::UDPATE_DIR)
        .arg("install")
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                let mut message = String::from("Fehler bei der Installation.");
                message.push_str(command_output_formater(&output).as_str());
                return Err(message);
            }
        }
        Err(err) => {
            return Err(format!("Fehler bei der Installation: {}", err));
        }
    }

    println!("Fertig!");
    println!("Die neuste Version ist jetzt installiert.");
    Ok(())
}
