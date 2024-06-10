#![allow(dead_code)]

use std::{
    fs::{read_to_string, remove_dir_all, rename, File},
    io::{BufRead, BufReader, Read},
    net::TcpStream,
    path::Path,
    process::{exit, Command, Output},
    sync::{Arc, Mutex},
};

use git2::Repository;

use crate::settings::{self, PID_FILE};

pub type ArcMutex<T> = Arc<Mutex<T>>;

#[macro_export]
macro_rules! trust_me_bro {
    ($($body:tt)*) => {
        unsafe {
            $($body)*
        }
    };
}

/// Wartet maximal `Duration` darauf, dass `Bedingung` true wird, returnt `true` wenn Bedingung vor
/// Ablauf der Zeit `true` wurde.
#[macro_export]
macro_rules! wait_with_timeout {
    ($closure:expr, $timeout:expr) => {{
        let start = std::time::Instant::now();
        while !$closure() && start.elapsed() < $timeout {}
        $closure()
    }};
}

/// Überprüft die Bedingung `condition` und wenn sie falsch ergibt printet `message` zu stderr und
/// exitet mit 1
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
pub fn read_pid_file() -> Result<String, String> {
    feddit_archivieren_assert(
        pid_file_exists(),
        "Versuche PID Datei zu lesen, sie existiert aber nicht.",
    );
    match BufReader::new(File::open(PID_FILE).unwrap())
        .lines()
        .filter_map(Result::ok)
        .next()
    {
        Some(line) => Ok(line),
        None => Err("Die PID Datei ist leer.".to_string()),
    }
}

/// Returnt ob der Daemon gerade läuft
pub fn daemon_running() -> bool {
    if pid_file_exists() {
        if let Ok(pid) = read_pid_file() {
            Path::new(&format!("/proc/{}", pid)).exists()
        } else {
            false
        }
    } else {
        false
    }
}

/// Ändert die Berechtigungen von `filepath` zu `mode`
pub fn chmod(filepath: &str, mode: &str) {
    run_command(Command::new("chmod").arg(mode).arg(filepath))
}

/// Returnt true wenn das Programm mit root gestartet wurde, sonst false
pub fn root() -> bool {
    users::get_current_uid() == 0
}

/// Führt einen Befehl aus und exitet mit einer Fehlermeldung sobald ein Fehler auftritt
pub fn run_command(command: &mut Command) {
    match command.output() {
        Ok(output) => {
            if !output.status.success() {
                eprintln!("Fehler bei einem Befehl:");
                eprintln!("{}", command_output_formater(&output));
                exit(1);
            }
        }
        Err(err) => {
            eprintln!("Fehler bei einem Befehl: {}", err);
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

fn get_update_version() -> String {
    let content = read_to_string(format!("{}/Cargo.toml", settings::UDPATE_DIR)).unwrap();
    let toml: toml::Value = content.parse().unwrap();
    toml.get("package")
        .and_then(|package| package.get("version"))
        .and_then(|version| version.as_str())
        .unwrap()
        .to_string()
}

fn get_current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Updatet das Programm
pub fn update(
    print_override: Option<fn(message: &str, streams: ArcMutex<Vec<TcpStream>>)>,
    print_args: Option<ArcMutex<Vec<TcpStream>>>,
) -> Result<(), String> {
    macro_rules! print_maybe_override {
        ($($e:expr), *) => {{
            let msg = &format!($($e), *);
            if print_override.is_some() {
                print_override.unwrap()(msg, print_args.clone().unwrap())
            } else {
                println!("{}", msg);
            }
        }};
    }

    let old_dir = Path::new(settings::UDPATE_DIR);
    let build_cache = old_dir.join("target");
    let mut build_cache_exists = build_cache.exists();

    if build_cache_exists {
        if Path::new(settings::UDPATE_CACHE_DIR).exists() {
            print_maybe_override!("Lösche {}...", settings::UDPATE_CACHE_DIR);
            if let Err(err) = remove_dir_all(settings::UDPATE_CACHE_DIR) {
                print_maybe_override!(
                    "Fehler beim Löschen von {}: {}",
                    settings::UDPATE_CACHE_DIR,
                    err
                );
            }
        }

        print_maybe_override!(
            "Bewege {:?} nach \"{}\"...",
            build_cache,
            settings::UDPATE_CACHE_DIR
        );

        if let Err(err) = rename(&build_cache, settings::UDPATE_CACHE_DIR) {
            print_maybe_override!(
                "Fehler beim Bewegen des Caches von {:?} zu {}: {}.",
                build_cache,
                settings::UDPATE_CACHE_DIR,
                err
            );

            print_maybe_override!("Ignoriere den Build Cache.");
            build_cache_exists = false;
        }
    }

    if old_dir.exists() {
        print_maybe_override!("Lösche {}...", settings::UDPATE_DIR);

        if let Err(err) = remove_dir_all(settings::UDPATE_DIR) {
            return Err(format!(
                "Fehler beim Löschen von {}: {}",
                settings::UDPATE_DIR,
                err
            ));
        }
    }

    print_maybe_override!(
        "Klone {} nach {}...",
        settings::GITHUB_LINK,
        settings::UDPATE_DIR
    );

    if let Err(err) = Repository::clone(settings::GITHUB_LINK, settings::UDPATE_DIR) {
        return Err(format!("Fehler beim Klonen: {}", err));
    };

    if settings::GIT_BRANCH != "main" {
        print_maybe_override!(
            "Wechsel von Branch main zu Branch {}...",
            settings::GIT_BRANCH
        );
        let success;
        match Command::new("git")
            .arg("checkout")
            .arg(settings::GIT_BRANCH)
            .current_dir(settings::UDPATE_DIR)
            .output()
        {
            Ok(output) => {
                success = output.status.success();
                if output.status.success() {
                    print_maybe_override!("Zu Branch {} gewechselt.", settings::GIT_BRANCH);
                } else {
                    print_maybe_override!(
                        "Fehler beim Auschecken von {} in {}: {}",
                        settings::GIT_BRANCH,
                        settings::UDPATE_DIR,
                        command_output_formater(&output)
                    );
                }
            }
            Err(err) => {
                success = false;
                print_maybe_override!(
                    "Fehler beim Auschecken von {} in {}: {}",
                    settings::GIT_BRANCH,
                    settings::UDPATE_DIR,
                    err
                );
            }
        }

        if !success {
            print_maybe_override!("Falle auf main zurück.");
        }
    }

    if build_cache_exists {
        print_maybe_override!(
            "Bewege \"{}\" nach {:?}...",
            settings::UDPATE_CACHE_DIR,
            build_cache
        );

        if let Err(err) = rename(settings::UDPATE_CACHE_DIR, &build_cache) {
            print_maybe_override!(
                "Fehler beim Bewegen des Caches von {} zu {:?}: {}.",
                settings::UDPATE_CACHE_DIR,
                build_cache,
                err
            );

            print_maybe_override!("Ignoriere den Build Cache.");
        }
    }

    print_maybe_override!("Fertig!");

    if get_current_version() == get_update_version() {
        print_maybe_override!("Bereits die neuste Version ({}).", get_current_version());
        return Ok(());
    }

    print_maybe_override!(
        "Neue Version gefunden: {} -> {}",
        get_current_version(),
        get_update_version()
    );
    print_maybe_override!("Compile den Source Code...");

    // Den Code mithilfe des Makefiles compilen und installieren
    match Command::new("make")
        .current_dir(settings::UDPATE_DIR)
        .arg("install")
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                return Err(format!(
                    "Fehler bei der Installation.\n{}",
                    command_output_formater(&output)
                ));
            }
        }
        Err(err) => {
            return Err(format!("Fehler bei der Installation: {}", err));
        }
    }

    print_maybe_override!("Fertig!");
    print_maybe_override!(
        "Die neuste Version ({}) ist jetzt installiert.",
        get_update_version()
    );
    print_maybe_override!("Update erfolgreich abgeschlossen.");
    Ok(())
}
