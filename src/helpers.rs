use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    net::TcpStream,
    path::Path,
    process::{exit, Command, Output},
};

use crate::settings::{self, PID_FILE};

#[allow(dead_code)]
pub fn feddit_archivieren_assert(condition: bool, message: &str) {
    if !condition {
        println!("{}", message);
        exit(1);
    }
}

pub fn pid_file_exists() -> bool {
    Path::new(PID_FILE).exists()
}

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

pub fn daemon_running() -> bool {
    if pid_file_exists() {
        Path::new(&format!(
            "/proc/{}",
            BufReader::new(File::open(PID_FILE).expect("Fehler beim Öffnen der PID Datei."))
                .lines()
                .next()
                .expect("Die PID Datei ist leer.")
                .expect("Die PID Datei ist korrupiert.")
        ))
        .exists()
    } else {
        false
    }
}

pub fn chmod(filepath: &str, mode: &str) {
    run_install_command(Command::new("chmod").arg(mode).arg(filepath))
}

pub fn root() -> bool {
    users::get_current_uid() == 0
}

pub fn run_install_command(command: &mut Command) {
    match command.output() {
        Ok(output) => {
            if !output.status.success() {
                println!("Fehler bei der Installation:");
                println!("{}", command_output_formater(&output));
                exit(1);
            }
        }
        Err(err) => {
            println!("Fehler bei der Installation: {}", err);
            exit(1);
        }
    }
}

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

#[allow(dead_code)]
pub fn get(filepath: &str) -> String {
    match File::open(filepath) {
        Ok(file) => {
            if let Some(line) = BufReader::new(file).lines().next() {
                line.unwrap_or_else(|err| format!("{}", err))
            } else {
                "Datei leer.".to_string()
            }
        }
        Err(err) => format!("{}", err),
    }
}

pub fn read_from_stream(stream: &mut TcpStream) -> String {
    let mut buf = [0; settings::TCP_BUFFER_SIZE];
    if let Err(err) = stream.read(&mut buf) {
        println!("Fehler beim Lesen aus einem TcpStream: {}", err);
    }
    to_rust_string(&buf)
}

pub fn to_rust_string(buf: &[u8; settings::TCP_BUFFER_SIZE]) -> String {
    let string = String::from_utf8_lossy(buf);
    string.trim_end_matches('\0').to_string()
}
