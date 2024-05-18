use daemonize::Daemonize;
use helpers::root;
use std::{
    fs::File,
    io::{ErrorKind, Write},
    net::{TcpListener, TcpStream},
    process::exit,
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::Duration,
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

    let recievers: Arc<Mutex<Vec<TcpStream>>> = Arc::new(Mutex::new(Vec::new()));

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
        let guard = recievers.clone();
        thread::spawn(move || match stream {
            Err(err) => {
                eprint(
                    &format!("Fehlerhafte Verbindung empfangen: {}", err),
                    &*guard.lock().unwrap(),
                );
                return;
            }
            Ok(mut stream) => {
                print(
                    &format!("Empfange Verbindung mit {}...", stream.peer_addr().unwrap()),
                    &*guard.lock().unwrap(),
                );
                let message = read_from_stream(&mut stream);

                print(
                    &format!("Nachricht: \"{}\"", message),
                    &*guard.lock().unwrap(),
                );

                match message.as_str() {
                    "ping" => {
                        print("Schreibe 'pong' in den stream", &*guard.lock().unwrap());
                        stream.write_all(b"pong").unwrap();
                    }
                    "stop" => {
                        print("Stoppe den Daemon.", &*guard.lock().unwrap());
                        shutdown_preperations(&*guard.lock().unwrap());
                        stream.write_all(b"ok").unwrap();
                        println!("Exite.");
                        exit(0);
                    }
                    "listen" => {
                        // TODO: implementieren
                        stream
                            .write_all(b"Achtung: Aktuell noch extrem unstable!")
                            .unwrap();

                        guard.lock().unwrap().push(stream);
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

fn print(message: &str, streams: &Vec<TcpStream>) {
    println!("{}", message);
    for mut stream in streams {
        stream.write(message.as_bytes()).unwrap();
    }
    sleep(Duration::from_millis(1));
}

fn eprint(message: &str, streams: &Vec<TcpStream>) {
    eprintln!("{}", message);
    for mut stream in streams {
        stream.write(message.as_bytes()).unwrap();
    }
}

/// Ändert die Berechtigungen einer Datei zu read-write für alle Nutzer
fn chmod_to_non_root(filepath: &str) {
    if root() {
        chmod(filepath, "666")
    }
}

/// Wird ausgeführt nachdem stop empfangen wurde
fn shutdown_preperations(streams: &Vec<TcpStream>) {
    for mut stream in streams {
        stream.write(b"Tschau :)").unwrap();
        stream.shutdown(std::net::Shutdown::Both).unwrap();
    }
}

/// Funktion die der Feddit-Thread ausführt
#[allow(dead_code)]
fn feddit() {
    todo!()
}
