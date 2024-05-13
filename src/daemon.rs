extern crate daemonize;

use chrono::Local;
use colored::{ColoredString, Colorize};
use core::fmt;
use daemonize::Daemonize;
use helpers::root;
use reqwest::{ClientBuilder, Method, Request, Url};
use std::{
    fs::File,
    io::{stdout, ErrorKind, Write},
    net::{TcpListener, TcpStream},
    process::exit,
    str::FromStr,
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::{Duration, Instant},
};
mod helpers;
mod settings;

use crate::{
    helpers::{chmod, daemon_running, pid_file_exists, read_from_stream},
    settings::{ERR_FILE, OUT_FILE, PID_FILE, SOCKET_FILE},
};

type Data = Arc<Mutex<(String, Vec<TcpStream>)>>;

macro_rules! print_formatted_to_streams {
    ($a:expr, $b:expr, $c:expr) => {
        if !_print_formatted_to_streams($a, $b, $c) {
            println!("Verbindung geschlossen.");
            return;
        }
        sleep(Duration::from_millis(10));
    };
}

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

    // Archive und Feddit spawnen
    let streams = Arc::new(Mutex::new(Vec::new()));
    let guard = streams.clone();
    let data: Data = Arc::new(Mutex::new((
        String::from(settings::FEDDIT_LINK),
        Vec::new(),
    )));
    let data_guard = data.clone();
    let streams_guard = guard.clone();
    feddit(streams_guard, data_guard).await;
    let data_guard = data.clone();
    let streams_guard = guard.clone();
    archive(streams_guard, data_guard).await;

    // Auf reinkommende Befehl hören
    for stream in listener.incoming() {
        let guard = streams.clone();
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

fn print_formatted(stream: &mut TcpStream, severity: &Severity, message: &str) -> bool {
    let start = format!(
        "[{}]\t[{}] ",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        severity
    );
    println!("{}{}", start, message);
    stream
        .write_all(
            ColoredString::from(format!("{}{}", start, severity.colored(message))).as_bytes(),
        )
        .is_ok()
}

fn _print_formatted_to_streams(
    streams: &Arc<Mutex<Vec<TcpStream>>>,
    severity: Severity,
    message: &str,
) -> bool {
    for mut stream in streams.lock().unwrap().iter_mut() {
        if !print_formatted(&mut stream, &severity, message) {
            return false;
        }
        stdout().flush().unwrap();
    }
    true
}

async fn feddit(streams: Arc<Mutex<Vec<TcpStream>>>, data: Data) {
    print_formatted_to_streams!(&streams, Severity::Error, "Hallo von Feddit!".into());
    let client_builder = ClientBuilder::new();
    let client = ClientBuilder::timeout(client_builder, Duration::from_secs(10))
        .build()
        .unwrap();
    let lock = data.lock().unwrap();
    let url = lock.0.clone();
    drop(lock);
    let request = Request::new(Method::GET, Url::from_str(url.as_str()).unwrap());
    let start = Instant::now();
    let content: String;
    loop {
        match client.execute(request.try_clone().unwrap()).await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    match response.text().await {
                        Ok(text) => {
                            content = text;
                            break;
                        }
                        Err(err) => {
                            print_formatted_to_streams!(
                                &streams,
                                Severity::Error,
                                format!("Fehler beim Encoden des HTML Bodys: {}", err).as_str()
                            );
                            shutdown_preperations();
                            exit(1);
                        }
                    }
                } else {
                    println!("{} antwortet mit statuscode {}.", request.url(), status);
                    if start.elapsed() > Duration::from_secs(1800) {
                        print_formatted_to_streams!(
                        &streams,
                        Severity::Error,
                        format!("Innerhalb von 30 Minuten hat sich das Problem nicht von selbst gelöst, exite mit Code 1.").as_str()
                    );
                        shutdown_preperations();
                        exit(1);
                    }
                }
            }
            Err(err) => {
                print_formatted_to_streams!(
                    &streams,
                    Severity::Warning,
                    format!("Fehler beim Versuch eine Request zu senden: {}", err).as_str()
                );
                if start.elapsed() > Duration::from_secs(600) {
                    print_formatted_to_streams!(
                        &streams,
                        Severity::Error,
                        format!("Innerhalb von 10 Minuten hat sich das Problem nicht von selbst gelöst, exite mit Code 1. ({})", err).as_str()
                    );
                    shutdown_preperations();
                    exit(1);
                }
            }
        }
    }
    print_formatted_to_streams!(
        &streams,
        Severity::Error,
        format!("Erfolgreich folgenden Body empfangen: {}", content).as_str()
    );
}

async fn archive(streams: Arc<Mutex<Vec<TcpStream>>>, data: Data) {}
