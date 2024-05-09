use std::{
    env::args,
    fs::create_dir,
    io::BufRead,
    path::Path,
    process::{exit, Command, Output},
};

use helpers::{daemon_running, feddit_archivieren_assert, read_pid_file};

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
            if users::get_current_uid() == 0 {
                if daemon_running() {
                    println!("Es läuft aktuell schon ein Daemon!");
                    exit(1);
                }

                if !Path::new(settings::RUN_DIR).exists() {
                    let result = create_dir(settings::RUN_DIR);
                    if result.is_err() {
                        println!(
                            "Fehler beim Erstellen von {}: {}",
                            settings::RUN_DIR,
                            result.unwrap_err()
                        );
                    }
                }

                copy_file("target/debug/daemon", settings::DAEMON_PATH);
                chmod(settings::DAEMON_PATH, "777");
                copy_file("target/debug/client", settings::CLIENT_PATH);
                chmod(settings::CLIENT_PATH, "777");

                println!("Installation erfolgreich!");
            } else {
                println!("Die Installation kopiert u.a. Dinge in /usr/bin, weshalb sie als root ausgeführt werden muss.");
                exit(1);
            }
        }
        "start" => {
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
            feddit_archivieren_assert(daemon_running(), "Der Daemon läuft nicht.");
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
        "update" => {
            if !Path::new(settings::UDPATE_TMP_DIR).exists() {
                let result = create_dir(settings::UDPATE_TMP_DIR);
                if result.is_err() {
                    println!(
                        "Fehler beim Erstellen von {}: {}",
                        settings::UDPATE_TMP_DIR,
                        result.unwrap_err()
                    );
                }
                match Command::new("git")
                    .arg("clone")
                    .arg(settings::GITHUB_LINK)
                    .arg(settings::UDPATE_TMP_DIR)
                    .output()
                {
                    Ok(output) => {
                        dbg!(&output);
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
                match Command::new("make")
                    .current_dir(settings::UDPATE_TMP_DIR)
                    .arg("clean")
                    .arg("install")
                    .output()
                {
                    Ok(output) => {
                        println!("{}", command_output_formater(&output));
                        if !output.status.success() {
                            println!("Fehler bei der Installation.");
                            exit(1);
                        }
                    }
                    Err(err) => {
                        println!("Fehler bei der Installation: {}", err);
                        exit(1);
                    }
                }
            }
        }
        _ => {
            println!("Unbekannter Befehl.");
            exit(1);
        }
    }
}

fn command_output_formater(output: &Output) -> String {
    let mut x = output
        .stdout
        .lines()
        .filter_map(Result::ok)
        .filter(|line| !line.is_empty())
        .collect::<Vec<String>>()
        .join("\n");
    x.push_str(
        output
            .stderr
            .lines()
            .filter_map(Result::ok)
            .filter(|line| !line.is_empty())
            .collect::<Vec<String>>()
            .join("\n")
            .as_str(),
    );
    x
}

fn run_install_command(command: &mut Command) {
    match command.output() {
        Ok(output) => {
            if !output.status.success() {
                println!("Fehler bei der Installation:");
                println!("{}", command_output_formater(&output));
                exit(1);
            }
        }
        Err(err) => {
            println!("Fehler bei der Installation: {}", err);
            exit(1);
        }
    }
}

fn copy_file(from: &str, to: &str) {
    run_install_command(Command::new("cp").arg(from).arg(to));
}

pub fn chmod(filepath: &str, mode: &str) {
    run_install_command(Command::new("chmod").arg(mode).arg(filepath))
}
