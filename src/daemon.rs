extern crate daemonize;

use daemonize::Daemonize;
use std::{fs::File, io::Write, net::TcpListener, process::exit};

mod helpers;
mod settings;

use crate::{
    helpers::{chmod, daemon_running, pid_file_exists},
    settings::{ERR_FILE, OUT_FILE, PID_FILE, SOCKET_FILE},
};

fn main() {
    // Überprüfen ob wir root sind
    if users::get_current_uid() != 0 {
        println!("Der Daemon muss als root gestartet werden!");
        exit(1);
    }

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
    let stdout = File::create(OUT_FILE).unwrap();
    let stderr = File::create(ERR_FILE).unwrap();

    let daemonize = Daemonize::new()
        .pid_file(PID_FILE)
        .working_directory(".")
        .stdout(stdout)
        .stderr(stderr);

    chmod(PID_FILE, "777");

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
