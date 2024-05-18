use std::{
    env::args,
    fs::{create_dir, remove_dir_all, remove_file, File},
    io::{stdout, BufRead, BufReader, Write},
    net::TcpStream,
    path::Path,
    process::{exit, Command},
    time::{Duration, Instant},
};

use helpers::{
    daemon_running, feddit_archivieren_assert, read_pid_file, root, run_install_command,
};

use crate::helpers::{chmod, command_output_formater, get, read_from_stream};

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

            remove_if_existing(settings::DAEMON_PATH);
            copy_file("target/debug/daemon", settings::DAEMON_PATH);
            if root() {
                chmod(settings::DAEMON_PATH, "777");
            }

            remove_if_existing(settings::CLIENT_PATH);
            copy_file("target/debug/client", settings::CLIENT_PATH);
            if root() {
                chmod(settings::CLIENT_PATH, "777");
            }

            println!("Installation erfolgreich!");
        }
        "start" => {
            create_run_dir();
            feddit_archivieren_assert(!daemon_running(), "Der Daemon läuft bereits.");
            let mut launch_command = Command::new(settings::DAEMON_PATH);
            match launch_command.output() {
                Ok(output) => {
                    if output.status.success() {
                        println!("Daemon erfolgreich gestartet!");
                        exit(0);
                    }
                    println!("Fehler beim Starten des Daemons:");
                    println!("{}", command_output_formater(&output));
                    exit(1);
                }
                Err(err) => {
                    println!("Fehler beim Starten des Daemons: {}", err);
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
            } else {
                println!("Der Bruder ist schon tot :( (Agavendicksaft Moment)")
            }
        }
        "update" => {
            feddit_archivieren_assert(!daemon_running(), "Der Daemon läuft gerade.");
            feddit_archivieren_assert(root(), "Du must root sein.");

            if let Err(message) = update() {
                println!("Fehler beim Updaten: ");
                println!("{}", message);
                exit(1);
            } else {
                println!("Update erfolgreich abgeschlossen.");
                exit(0);
            }
        }
        "update_local" => {
            if daemon_running() {
                kill_daemon();
            }

            feddit_archivieren_assert(root(), "Du must root sein.");

            match Command::new("make").arg("clean").arg("install").output() {
                Ok(output) => {
                    if !output.status.success() {
                        println!("Fehler bei der Installation.");
                        println!("{}", command_output_formater(&output));
                        exit(1);
                    }
                }
                Err(err) => {
                    println!("Fehler bei der Installation: {}", err);
                    exit(1);
                }
            }
            println!("Lokales Update erfolgreich abgeschlossen.");
        }
        "clean" => {
            if Path::new(settings::RUN_DIR).exists() {
                if let Err(error) = remove_dir_all(settings::RUN_DIR) {
                    println!("Fehler beim Löschen von {}: {}", settings::RUN_DIR, error);
                }
            }
            if Path::new(settings::UDPATE_DIR).exists() {
                if let Err(error) = remove_dir_all(settings::UDPATE_DIR) {
                    println!(
                        "Fehler beim Löschen von {}: {}",
                        settings::UDPATE_DIR,
                        error
                    );
                }
            }
        }
        "info" => {
            feddit_archivieren_assert(daemon_running(), "Der Daemon läuft nicht.");
            println!("Der Daemon läuft.");
            println!("Port:\t{}", get(settings::SOCKET_FILE));
            println!("PID:\t{}", get(settings::PID_FILE));
        }
        "logs_static" => {
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
                    println!("Fehler beim Lesen von {}: {}", settings::ERR_FILE, err);
                }
            }
        }
        "checkhealth" => {
            feddit_archivieren_assert(daemon_running(), "Der Daemon läuft nicht.");

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
            let start = Instant::now();
            loop {
                if !daemon_running() || start.elapsed() > Duration::from_secs(1) {
                    break;
                }
            }
            feddit_archivieren_assert(
                !daemon_running(),
                "Der Daemon hat eine Bestätigung gesendet, läuft aber immer noch.",
            );
            println!("Der Daemon wurde erfolgreich beendet!");
        }
        "listen" => {
            feddit_archivieren_assert(daemon_running(), "Der Daemon läuft nicht.");
            let mut stream = send_to_daemon("listen");
            loop {
                let response = read_from_stream(&mut stream);
                if response.is_empty() {
                    println!("Der Daemon hat die Verbindung geschlossen.");
                    exit(0);
                }
                println!("{}", response);
                stdout().flush().unwrap();
            }
        }
        _ => {
            println!("Unbekannter Befehl.");
            exit(1);
        }
    }
}

fn copy_file(from: &str, to: &str) {
    run_install_command(Command::new("cp").arg(from).arg(to));
}

fn create_run_dir() {
    if !Path::new(settings::RUN_DIR).exists() {
        let result = create_dir(settings::RUN_DIR);
        if result.is_err() {
            println!(
                "Fehler beim Erstellen von {}: {}",
                settings::RUN_DIR,
                result.unwrap_err()
            );
            exit(1);
        }
    }
}

fn remove_if_existing(filepath: &str) {
    if Path::new(filepath).exists() {
        if let Err(err) = remove_file(filepath) {
            println!("Fehler beim Löschen von {}: {}", filepath, err);
        }
    }
}

fn kill_daemon() {
    feddit_archivieren_assert(daemon_running(), "Der Daemon läuft nicht.");
    feddit_archivieren_assert(root(), "Du bist nicht root.");

    create_run_dir();

    match Command::new("kill").arg(read_pid_file()).output() {
        Ok(output) => {
            if !output.status.success() {
                println!("Fehler beim Killen des Daemons:");
                println!("{}", command_output_formater(&output));
            } else {
                println!("Daemon erfolgreich gekillt.");
            }
        }
        Err(err) => {
            println!("Fehler beim Killen des Daemons: {}", err);
        }
    }
}

fn send_to_daemon(message: &str) -> TcpStream {
    let mut stream = match TcpStream::connect(get(settings::SOCKET_FILE)) {
        Ok(stream) => stream,
        Err(err) => {
            println!(
                "Fehler beim Verbinden mit {}: {}",
                settings::SOCKET_FILE,
                err
            );
            exit(1);
        }
    };
    if let Err(err) = stream.write_all(message.as_bytes()) {
        println!("Fehler beim Senden an den Daemon: {}", err);
        exit(1);
    }
    stream
}

fn update() -> Result<(), String> {
    if !Path::new(settings::UDPATE_DIR).exists() {
        let result = create_dir(settings::UDPATE_DIR);
        if result.is_err() {
            println!(
                "Fehler beim Erstellen von {}: {}",
                settings::UDPATE_DIR,
                result.unwrap_err()
            );
        }
        match Command::new("git")
            .arg("clone")
            .arg(settings::GITHUB_LINK)
            .arg(settings::UDPATE_DIR)
            .output()
        {
            Ok(output) => {
                println!("{}", command_output_formater(&output));
                if !output.status.success() {
                    println!("Fehler beim Klonen.");
                    exit(1);
                }
            }
            Err(err) => {
                println!(
                    "Fehler beim Klonen von {} nach {}: {}",
                    settings::GITHUB_LINK,
                    settings::UDPATE_DIR,
                    err
                );
            }
        }
    } else {
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
