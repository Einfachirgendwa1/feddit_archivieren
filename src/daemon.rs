extern crate daemonize;

use daemonize::Daemonize;
use helpers::root;
use std::{
    fs::File,
    io::{ErrorKind, Read, Write},
    net::TcpListener,
    process::exit,
    thread,
};

mod helpers;
mod settings;

use crate::{
    helpers::{chmod, daemon_running, pid_file_exists, to_rust_string},
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
                let mut buf = [0; 1024];
                if let Err(err) = stream.read(&mut buf) {
                    eprintln!("Fehler beim Lesen aus einer TCP Connection: {}", err);
                    return;
                }

                let message = to_rust_string(&buf);

                println!("Nachricht: \"{}\"", message);

                if message == "ping" {
                    println!("Schreibe 'pong' in den stream");
                    stream.write_all(b"pong").unwrap();
                }
            }
        });
    }
}

pub fn chmod_to_non_root(filepath: &str) {
    if root() {
        chmod(filepath, "666")
    }
}
