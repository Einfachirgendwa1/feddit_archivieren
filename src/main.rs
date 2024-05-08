extern crate daemonize;

use daemonize::Daemonize;
use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    path::Path,
    process::exit,
};

fn main() {
    // Überprüfen ob bereits ein Daemon läuft
    let pidfile = Path::new("daemon.pid");
    if pidfile.exists() {
        println!("PID Datei existiert.");

        let pid =
            BufReader::new(File::open("daemon.pid").expect("Fehler beim Öffnen der PID Datei."))
                .lines()
                .next()
                .expect("Die PID Datei ist leer.")
                .expect("Die PID Datei ist korrupiert.");

        if Path::new(&format!("/proc/{}", pid)).exists() {
            println!("Der Daemon läuft bereits mit der PID {}!", pid);
        } else {
            println!(
                "Der dort aufgeführte Prozess ({}) scheint nicht zu existieren.",
                pid
            );
        }

        println!("Stoppe den Versuch einen neuen Daemon zu starten um Datenverlust zu vermeiden.");
        println!("INFO: Starte mit --force um das Starten zu erzwingen.");
        exit(1);
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
