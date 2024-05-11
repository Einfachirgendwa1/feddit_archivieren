use std::{
    env::args,
    fs::{create_dir, remove_dir_all, remove_file, File},
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    path::Path,
    process::{exit, Command},
};

use helpers::{
    daemon_running, feddit_archivieren_assert, read_pid_file, root, run_install_command,
};

use crate::helpers::{chmod, command_output_formater, get, to_rust_string};

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
        "maybe_kill" => {
            if daemon_running() {
                kill_daemon();
            }
        }
        "update" => {
            feddit_archivieren_assert(!daemon_running(), "Der Daemon läuft gerade.");
            feddit_archivieren_assert(root(), "Du must root sein.");

            if !Path::new(settings::UDPATE_TMP_DIR).exists() {
                let result = create_dir(settings::UDPATE_TMP_DIR);
                if result.is_err() {
                    println!(
                        "Fehler beim Erstellen von {}: {}",
                        settings::UDPATE_TMP_DIR,
                        result.unwrap_err()
                    );
                }
            } else {
                remove_dir_all(settings::UDPATE_TMP_DIR).expect(
                    format!("Fehler beim Löschen von {}.", settings::UDPATE_TMP_DIR).as_str(),
                );
            }
            match Command::new("git")
                .arg("clone")
                .arg(settings::GITHUB_LINK)
                .arg(settings::UDPATE_TMP_DIR)
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
                        settings::UDPATE_TMP_DIR,
                        err
                    );
                }
            }
            println!("Fertig.");
            match Command::new("make")
                .current_dir(settings::UDPATE_TMP_DIR)
                .arg("clean")
                .arg("install")
                .output()
            {
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
            remove_dir_all(settings::UDPATE_TMP_DIR)
                .expect(format!("Fehler beim Löschen von {}.", settings::UDPATE_TMP_DIR).as_str());

            println!("Update erfolgreich abgeschlossen.");
        }
        "update_local" => {
            feddit_archivieren_assert(!daemon_running(), "Der Daemon läuft gerade.");
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
        }
        "info" => {
            feddit_archivieren_assert(daemon_running(), "Der Daemon läuft nicht.");
            println!("Der Daemon läuft.");
            println!("Port:\t{}", get(settings::SOCKET_FILE));
            println!("PID:\t{}", get(settings::PID_FILE));
        }
        "logs_static" => {
            println!("STDOUT:");
            match File::open(settings::OUT_FILE) {
                Ok(file) => {
                    let mut iterator = BufReader::new(&file).lines();
                    while let Some(line) = iterator.next() {
                        println!(
                            "{}",
                            line.unwrap_or("<FEHLER BEIM LESEN DIESER ZEILE>".to_string())
                        );
                    }
                }
                Err(err) => {
                    println!("Fehler beim Lesen von {}: {}", settings::OUT_FILE, err);
                }
            }
            println!("<Ende>");
            println!("STDERR:");
            match File::open(settings::ERR_FILE) {
                Ok(file) => {
                    let mut iterator = BufReader::new(&file).lines();
                    while let Some(line) = iterator.next() {
                        println!(
                            "{}",
                            line.unwrap_or("<FEHLER BEIM LESEN DIESER ZEILE>".to_string())
                        );
                    }
                }
                Err(err) => {
                    println!("Fehler beim Lesen von {}: {}", settings::ERR_FILE, err);
                }
            }
            println!("<Ende>");
        }
        "checkhealth" => {
            feddit_archivieren_assert(daemon_running(), "Der Daemon läuft nicht.");

            println!("Versuche Daten in den Stream zu schreiben.");
            let mut stream = TcpStream::connect(get(settings::SOCKET_FILE)).unwrap();
            stream.write_all(b"ping").unwrap();
            println!("Fertig.");

            println!("Versuche Daten aus dem Stream zu empfangen.");
            let mut buf = [0; 1024];
            stream.read(&mut buf).unwrap();

            let message = to_rust_string(&buf);

            feddit_archivieren_assert(
                message == "pong",
                format!("Nachricht pong erwartet, '{}' empfangen.", message).as_str(),
            );
            println!("Nachricht pong erfolgreich empfangen!");
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
