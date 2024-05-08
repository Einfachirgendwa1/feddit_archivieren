extern crate daemonize;

use daemonize::Daemonize;
use std::{fs::File, io::Write, net::TcpListener, process::exit};

mod helpers;

use crate::helpers::{daemon_running, pid_file_exists};

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
    let stdout = File::create("daemon.out").unwrap();
    let stderr = File::create("daemon.err").unwrap();

    let daemonize = Daemonize::new()
        .pid_file("daemon.pid")
        .working_directory(".")
        .stdout(stdout)
        .stderr(stderr);

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

    // Unsere Socketadresse in daemon.sck schreiben
    let mut socketfile = File::create("daemon.sck").unwrap();
    socketfile
        .write_all(socket.to_string().as_bytes())
        .expect("Fehler beim Schreiben ins Socketfile.");

    println!("Socketadresse in eine Datei geschrieben.");

    for stream in listener.incoming() {
        match stream {
            Err(e) => eprintln!("Fehlerhaften Stream empfangen: {}", e),
            Ok(tcp_stream) => {
                println!("Neue Verbindung: {}", tcp_stream.peer_addr().unwrap());
            }
        }
    }
}
