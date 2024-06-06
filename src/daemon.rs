use colored::{ColoredString, Colorize};
use daemonize::Daemonize;
use helpers::root;
use std::{
    fs::File,
    io::{BufWriter, ErrorKind, Write},
    net::{TcpListener, TcpStream},
    process::exit,
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::Duration,
};
mod helpers;
mod settings;

use crate::{
    helpers::{chmod, daemon_running, read_from_stream, update},
    settings::{ERR_FILE, OUT_FILE, PID_FILE, POST_FILE, SOCKET_FILE, URL_FILE},
};

macro_rules! unwrap_mutex_save {
    ($e:expr) => {
        *match $e.lock() {
            Ok(lock) => lock,
            Err(poison) => {
                println!("Der Mutex ist gepoisent: {}", poison);
                println!("Ignoriere den Error vorerst, dies scheint jedoch Anzeichen für einen Bug im Code zu sein.");
                poison.into_inner()
            }
        }
    }
}

/// streams, guard, running_guard, url, posts_guard, feddit_guard, archive_guard
macro_rules! shutdown {
    ($stream:expr, $guard:expr, $running_guard:expr, $url:expr, $posts_guard:expr, $feddit_guard:expr, $archive_guard:expr) => {
        print("Stoppe den Daemon.", $guard.clone());
        unwrap_mutex_save!($running_guard) = false;
        shutdown_preperations(&*$guard.lock().unwrap(), $url, $posts_guard);
        wait_with_timeout!(
            || unwrap_mutex_save!($feddit_guard).is_finished().clone()
                && unwrap_mutex_save!($archive_guard).is_finished().clone(),
            Duration::from_secs(1)
        );
        $stream.write_all(b"ok").unwrap();
        println!("Exite.");
        exit(0);
    };
}

fn main() {
    let url = settings::FEDDIT_LINK;
    let posts: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));

    // Überprüfen ob bereits ein Daemon läuft
    if daemon_running() {
        println!("Es läuft bereits ein Daemon!");
        println!("Stoppe den Versuch einen neuen Daemon zu starten um Datenverlust zu vermeiden.");
        println!("Starte mit --force um das Starten zu erzwingen.");
        exit(1);
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

    File::create(URL_FILE).unwrap();
    File::create(POST_FILE).unwrap();
    File::create(PID_FILE).unwrap();
    let stderr = File::create(ERR_FILE).unwrap();

    let daemonize = Daemonize::new()
        .pid_file(PID_FILE)
        .working_directory(".")
        .stdout(stdout)
        .stderr(stderr);

    chmod_to_non_root(OUT_FILE);
    chmod_to_non_root(ERR_FILE);
    chmod_to_non_root(PID_FILE);
    chmod_to_non_root(URL_FILE);
    chmod_to_non_root(POST_FILE);

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

    let running = Arc::new(Mutex::new(true));

    let guard = running.clone();
    let archive = Arc::new(Mutex::new(thread::spawn(|| archive(guard))));
    let guard = running.clone();
    let feddit = Arc::new(Mutex::new(thread::spawn(|| feddit(guard))));

    // Update Thread spawnen
    let guard = recievers.clone();
    thread::spawn(move || loop {
        print(
            &format!(
                "Warte {} Sekunden vor der nächsten Updateüberprüfung...",
                settings::UPDATE_FETCH_DELAY.as_secs()
            ),
            guard.clone(),
        );
        sleep(settings::UPDATE_FETCH_DELAY);
        print("Update...", guard.clone());
        if let Err(err) = update(Some(print), Some(guard.clone())) {
            eprint(format!("{}", err).as_str(), guard.clone());
        }
    });

    // Auf reinkommende Befehl hören
    for stream in listener.incoming() {
        let guard = recievers.clone();
        let posts_guard = posts.clone();
        let running_guard = running.clone();
        let feddit_guard = feddit.clone();
        let archive_guard = archive.clone();
        thread::spawn(move || match stream {
            Err(err) => {
                eprint(
                    &format!("Fehlerhafte Verbindung empfangen: {}", err),
                    guard.clone(),
                );
                return;
            }
            Ok(mut stream) => {
                print(
                    &format!("Empfange Verbindung mit {}...", stream.peer_addr().unwrap()),
                    guard.clone(),
                );

                let message = read_from_stream(&mut stream);

                print(&format!("Nachricht: \"{}\"", message), guard.clone());

                match message.as_str() {
                    "ping" => {
                        print("Schreibe 'pong' in den stream", guard);
                        stream.write_all(b"pong").unwrap();
                    }
                    "restart" => {
                        print("restart", guard.clone());
                        stream.write_all(b"ok").unwrap();
                        shutdown!(
                            stream,
                            guard,
                            running_guard,
                            url,
                            posts_guard,
                            feddit_guard,
                            archive_guard
                        );
                    }
                    "stop" => {
                        shutdown!(
                            stream,
                            guard,
                            running_guard,
                            url,
                            posts_guard,
                            feddit_guard,
                            archive_guard
                        );
                    }
                    "listen" => {
                        stream.write_all(b"Hallo!").unwrap();

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

fn print(message: &str, streams: Arc<Mutex<Vec<TcpStream>>>) {
    println!("{}", message);

    let mut streams_override_idxs = Vec::new();

    for (index, mut stream) in unwrap_mutex_save!(streams).iter().enumerate() {
        if let Err(err) = stream.write(message.as_bytes()) {
            eprintln!("Fehler beim Schreiben in einen Stream: {}", err);
        } else {
            streams_override_idxs.push(index);
        }
    }

    let new: Vec<TcpStream> = unwrap_mutex_save!(streams)
        .drain(..)
        .enumerate()
        .filter(|(idx, _)| streams_override_idxs.contains(idx))
        .map(|(_, stream)| stream)
        .collect();
    unwrap_mutex_save!(streams) = new;

    sleep(Duration::from_millis(1));
}

fn eprint(message: &str, streams: Arc<Mutex<Vec<TcpStream>>>) {
    eprintln!("{}", message);

    let mut streams_override_idxs = Vec::new();

    for (index, mut stream) in unwrap_mutex_save!(streams).iter().enumerate() {
        if let Err(err) = stream.write(message.as_bytes()) {
            eprintln!("Fehler beim Schreiben in einen Stream: {}", err);
        } else {
            streams_override_idxs.push(index);
        }
    }

    let new: Vec<TcpStream> = unwrap_mutex_save!(streams)
        .drain(..)
        .enumerate()
        .filter(|(idx, _)| streams_override_idxs.contains(idx))
        .map(|(_, stream)| stream)
        .collect();
    unwrap_mutex_save!(streams) = new;

    sleep(Duration::from_millis(1));
}

/// Ändert die Berechtigungen einer Datei zu read-write für alle Nutzer
fn chmod_to_non_root(filepath: &str) {
    if root() {
        chmod(filepath, "666")
    }
}

/// Wird ausgeführt nachdem stop empfangen wurde
fn shutdown_preperations(streams: &Vec<TcpStream>, url: &str, posts: Arc<Mutex<Vec<i32>>>) {
    for mut stream in streams {
        stream.write(b"Tschau :)").unwrap();
        stream.shutdown(std::net::Shutdown::Both).unwrap();
    }

    if let Err(err) = save(url, unwrap_mutex_save!(posts).to_vec()) {
        println!("Fehler beim Speichern: {}", err);
    }
}

/// Speichert den aktuellen Fortschritt
fn save(url: &str, posts: Vec<i32>) -> Result<(), std::io::Error> {
    let mut url_file = File::create(settings::URL_FILE)?;
    let post_file = File::create(settings::POST_FILE)?;

    url_file.write_all(url.as_bytes())?;

    let mut writer = BufWriter::new(post_file);
    for post in posts {
        writer.write_all(post.to_string().as_bytes())?;
    }

    Ok(())
}

/// Funktion die vom Archive-Thread ausgeführt wird
fn archive(running: Arc<Mutex<bool>>) {
    loop {
        if unwrap_mutex_save!(running) == false {
            return;
        }
        sleep(Duration::from_millis(50));
    }
}

/// Funktion die vom Feddit-Thread ausgeführt wird
fn feddit(running: Arc<Mutex<bool>>) {
    loop {
        if unwrap_mutex_save!(running) == false {
            return;
        }
        sleep(Duration::from_millis(50));
    }
}
