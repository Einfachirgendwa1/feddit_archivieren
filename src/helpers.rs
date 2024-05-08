use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    process::exit,
};

#[allow(dead_code)]
pub fn feddit_archivieren_assert(condition: bool, message: &str) {
    if !condition {
        println!("{}", message);
        exit(1);
    }
}

pub fn pid_file_exists() -> bool {
    Path::new("daemon.pid").exists()
}

#[allow(dead_code)]
pub fn read_pid_file() -> String {
    feddit_archivieren_assert(
        pid_file_exists(),
        "Versuche PID Datei zu lesen, sie existiert aber nicht.",
    );
    BufReader::new(File::open("daemon.pid").unwrap())
        .lines()
        .filter_map(Result::ok)
        .next()
        .unwrap()
}

pub fn daemon_running() -> bool {
    if pid_file_exists() {
        Path::new(&format!(
            "/proc/{}",
            BufReader::new(File::open("daemon.pid").expect("Fehler beim Öffnen der PID Datei."))
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
