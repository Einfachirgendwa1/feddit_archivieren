use std::{
    env::args,
    io::BufRead,
    process::{exit, Command, Output},
};

use helpers::{feddit_archivieren_assert, read_pid_file};

use crate::helpers::daemon_running;

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

                run_install_command(
                    Command::new("cp")
                        .arg("target/debug/daemon")
                        .arg(settings::DAEMON_PATH),
                );
                run_install_command(
                    Command::new("cp")
                        .arg("target/debug/client")
                        .arg(settings::CLIENT_PATH),
                );
                println!("Installation erfolgreich!");
            } else {
                println!("Die Installation kopiert u.a. Dinge in /usr/bin, weshalb sie als root ausgeführt werden muss.");
                exit(1);
            }
        }
        "start" => {
            let mut launch_command = Command::new(settings::DAEMON_PATH);
            match launch_command.output() {
                Ok(output) => {
                    if output.status.success() {
                        println!("Daemon erfolgreich gestartet!");
                        exit(0);
                    }
                    println!("Fehler beim Starten des Daemons:");
                    println!("{}", command_error_formater(output));
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
                        println!("{}", command_error_formater(output));
                    } else {
                        println!("Daemon erfolgreich gekillt.");
                    }
                }
                Err(err) => {
                    println!("Fehler beim Killen des Daemons: {}", err);
                }
            }
        }
        _ => {
            println!("Unbekannter Befehl.");
            exit(1);
        }
    }
}

fn command_error_formater(output: Output) -> String {
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
                println!("{}", command_error_formater(output));
                exit(1);
            }
        }
        Err(err) => {
            println!("Fehler bei der Installation: {}", err);
            exit(1);
        }
    }
}
