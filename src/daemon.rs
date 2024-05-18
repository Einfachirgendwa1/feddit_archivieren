use daemonize::Daemonize;
use helpers::root;
use std::{
    fs::File,
    io::{ErrorKind, Write},
    net::TcpListener,
    process::exit,
    thread::{self},
};
mod helpers;
mod settings;

use crate::{
    helpers::{chmod, daemon_running, pid_file_exists, read_from_stream},
    settings::{ERR_FILE, OUT_FILE, PID_FILE, SOCKET_FILE},
};

#[tokio::main]
async fn main() {
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

    // TODO: Archive und Feddit spawnen

    // Auf reinkommende Befehl hören
    for stream in listener.incoming() {
        thread::spawn(move || match stream {
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
                        // TODO: implementieren
                        stream
                            .write_all(b"Noch nicht implementiert, sorry :(")
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

/// Ändert die Berechtigungen einer Datei zu read-write für alle Nutzer
fn chmod_to_non_root(filepath: &str) {
    if root() {
        chmod(filepath, "666")
    }
}

/// Wird ausgeführt nachdem stop empfangen wurde
fn shutdown_preperations() {}
