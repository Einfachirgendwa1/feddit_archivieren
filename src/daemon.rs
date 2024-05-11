extern crate daemonize;

use chrono::Local;
use colored::{ColoredString, Colorize};
use core::fmt;
use daemonize::Daemonize;
use helpers::root;
use std::{
    fs::File,
    io::{ErrorKind, Write},
    net::TcpListener,
    process::exit,
    thread,
};

mod helpers;
mod settings;

use crate::{
    helpers::{chmod, daemon_running, pid_file_exists, read_from_stream},
    settings::{ERR_FILE, OUT_FILE, PID_FILE, SOCKET_FILE},
};

fn main() {
    // Überprüfen ob bereits ein Daemon läuft
    if pid_file_exists() {
        println!("PID Datei existiert.");
        if daemon_running() {
            println!(
                "Stoppe den Versuch einen neuen Daemon zu starten um Datenverlust zu vermeiden."
            );
            // TODO: println!("Starte mit --force um das Starten zu erzwingen.");
            exit(1);
        }
    }

    // Den Daemon erstellen und starten
    let stdout = match File::create(OUT_FILE) {
        Ok(stdout) => stdout,
        Err(err) => {
            if err.kind() == ErrorKind::PermissionDenied {
                println!("Die erste Installation muss als root ausgeführt werden.");
            } else {
                dbg!(err);
            }
            exit(1);
        }
    };
    let stderr = File::create(ERR_FILE).unwrap();

    File::create(PID_FILE).unwrap();

    let daemonize = Daemonize::new()
        .pid_file(PID_FILE)
        .working_directory(".")
        .stdout(stdout)
        .stderr(stderr);

    chmod_to_non_root(OUT_FILE);
    chmod_to_non_root(ERR_FILE);
    chmod_to_non_root(PID_FILE);

    match daemonize.start() {
        Ok(_) => println!("Daemon erfolgreich gestartet."),
        Err(e) => eprintln!("Error, {}", e),
    }

    // An einen Socket binden
    let listener =
        TcpListener::bind("127.0.0.1:0").expect("Fehler beim Binden des Daemons an einen Socket.");

    let socket = listener
        .local_addr()
        .expect("Fehler beim Holen der Socket Adresse.");

    println!("Erfolgreich an einen Socket gebunden.");

    // Unsere Socketadresse ins Socketfile schreiben
    let mut socketfile = File::create(SOCKET_FILE).unwrap();
    chmod_to_non_root(SOCKET_FILE);
    socketfile
        .write_all(socket.to_string().as_bytes())
        .expect("Fehler beim Schreiben ins Socketfile.");

    println!("Socketadresse in eine Datei geschrieben.");

    for stream in listener.incoming() {
        thread::spawn(|| match stream {
            Err(err) => {
                eprintln!("Fehlerhafte Verbindung empfangen: {}", err);
                return;
            }
            Ok(mut stream) => {
                println!("Empfange Verbindung mit {}...", stream.peer_addr().unwrap());
                let message = read_from_stream(&mut stream);

                println!("Nachricht: \"{}\"", message);

                match message.as_str() {
                    "ping" => {
                        println!("Schreibe 'pong' in den stream");
                        stream.write_all(b"pong").unwrap();
                    }
                    "stop" => {
                        println!("Stoppe den Daemon.");
                        shutdown_preperations();
                        stream.write_all(b"ok").unwrap();
                        println!("Exite.");
                        exit(0);
                    }
                    "listen" => {
                        stream
                            .write_all(
                                print_formatted(Severity::Info, "Hallo :)")
                                    .to_string()
                                    .as_bytes(),
                            )
                            .unwrap();
                        stream
                            .write_all(
                                print_formatted(Severity::Warning, "Das ist ein Warning :)")
                                    .to_string()
                                    .as_bytes(),
                            )
                            .unwrap();
                        stream
                            .write_all(
                                print_formatted(Severity::Error, "Das ist ein Error :)")
                                    .to_string()
                                    .as_bytes(),
                            )
                            .unwrap();
                    }
                    _ => {
                        println!("Unbekannter Befehl.");
                        stream.write_all(b"unknown").unwrap();
                    }
                }
            }
        });
    }
}

fn chmod_to_non_root(filepath: &str) {
    if root() {
        chmod(filepath, "666")
    }
}

fn shutdown_preperations() {}

enum Severity {
    Info,
    Warning,
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match *self {
            Severity::Info => "Info",
            Severity::Warning => "Warning",
            Severity::Error => "Error",
        })
    }
}

impl Severity {
    fn colored(self: &Self, message: &str) -> ColoredString {
        match *self {
            Severity::Info => message.cyan(),
            Severity::Warning => message.yellow(),
            Severity::Error => message.red(),
        }
    }
}

fn print_formatted(severity: Severity, message: &str) -> ColoredString {
    let start = format!(
        "[{}]\t[{}] ",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        severity
    );
    print!("{}{}", start, message);
    ColoredString::from(format!("{}{}", start, severity.colored(message)))
}
