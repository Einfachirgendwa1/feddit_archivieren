use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    net::TcpStream,
    path::Path,
    process::{exit, Command, Output},
};

use crate::settings::{self, PID_FILE};

/// Überprüft die Bedingung `condition` und wenn sie falsch ergibt printet `message` zu stderr und
/// exitet mit 1
#[allow(dead_code)]
pub fn feddit_archivieren_assert(condition: bool, message: &str) {
    if !condition {
        eprintln!("{}", message);
        exit(1);
    }
}

/// Returnt true wenn PID_FILE existiert, false wenn nicht
pub fn pid_file_exists() -> bool {
    Path::new(PID_FILE).exists()
}

/// Returnt den Inhalt von PID_FILE, wenn es nicht existiert exitet mit 1
#[allow(dead_code)]
pub fn read_pid_file() -> String {
    feddit_archivieren_assert(
        pid_file_exists(),
        "Versuche PID Datei zu lesen, sie existiert aber nicht.",
    );
    BufReader::new(File::open(PID_FILE).unwrap())
        .lines()
        .filter_map(Result::ok)
        .next()
        .unwrap()
}

/// Returnt ob der Daemon gerade läuft
pub fn daemon_running() -> bool {
    if pid_file_exists() {
        Path::new(&format!("/proc/{}", read_pid_file())).exists()
    } else {
        false
    }
}

/// Ändert die Berechtigungen von `filepath` zu `mode`
pub fn chmod(filepath: &str, mode: &str) {
    run_install_command(Command::new("chmod").arg(mode).arg(filepath))
}

/// Returnt true wenn das Programm mit root gestartet wurde, sonst false
pub fn root() -> bool {
    users::get_current_uid() == 0
}

/// Führt einen Befehl aus und exitet mit einer Fehlermeldung sobald ein Fehler auftritt
pub fn run_install_command(command: &mut Command) {
    match command.output() {
        Ok(output) => {
            if !output.status.success() {
                eprintln!("Fehler bei der Installation:");
                eprintln!("{}", command_output_formater(&output));
                exit(1);
            }
        }
        Err(err) => {
            eprintln!("Fehler bei der Installation: {}", err);
            exit(1);
        }
    }
}

/// Nimmt einen Output eines Commands und returnt dann einen String welcher sowohl stderr als auch
/// stdout enthält
pub fn command_output_formater(output: &Output) -> String {
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

/// Liest eine Zeile aus einer Datei. Returnt eine Fehlermeldung wenn etwas schiefläuft.
#[allow(dead_code)]
pub fn get(filepath: &str) -> String {
    match File::open(filepath) {
        Ok(file) => {
            if let Some(line) = BufReader::new(file).lines().next() {
                match line {
                    Ok(line) => line,
                    Err(err) => err.to_string(),
                }
            } else {
                "Datei leer.".into()
            }
        }
        Err(err) => err.to_string(),
    }
}

/// Liest eine Nachricht aus einem TcpStream und returnt sie.
pub fn read_from_stream(stream: &mut TcpStream) -> String {
    let mut buf = [0; settings::TCP_BUFFER_SIZE];
    if let Err(err) = stream.read(&mut buf) {
        println!("Fehler beim Lesen aus einem TcpStream: {}", err);
    }
    to_rust_string(&buf)
}

/// Nimmt einen Buffer an u8 und macht daraus einen String
pub fn to_rust_string(buf: &[u8; settings::TCP_BUFFER_SIZE]) -> String {
    let string = String::from_utf8_lossy(buf);
    string.trim_end_matches('\0').into()
}
