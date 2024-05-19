use std::{
    fs::{read_dir, read_to_string, File},
    io::{BufRead, BufReader, Read},
    net::TcpStream,
    path::Path,
    process::{exit, Command, Output},
    sync::{Arc, Mutex},
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
    print_override: Option<fn(message: &str, streams: Arc<Mutex<Vec<TcpStream>>>)>,
    print_args: Option<Arc<Mutex<Vec<TcpStream>>>>,
) -> Result<(), String> {
    macro_rules! print_maybe_override {
        ($e:expr) => {
            if print_override.is_some() {
                print_override.unwrap()($e, print_args.clone().unwrap())
            } else {
                println!("{}", $e);
            }
        };
    }

    if !Path::new(settings::UDPATE_DIR).exists()
        || read_dir(settings::UDPATE_DIR).unwrap().next().is_none()
    {
        // Wenn das Verzeichnis noch nicht existiert, den Code dahinklonen
        print_maybe_override!(format!(
            "Klone {} nach {}...",
            settings::GITHUB_LINK,
            settings::UDPATE_DIR
        )
        .as_str());

        match Command::new("git")
            .arg("clone")
            .arg(settings::GITHUB_LINK)
            .arg(settings::UDPATE_DIR)
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    return Err(format!(
                        "Fehler beim Klonen von {} nach {}:\n{}",
                        settings::GITHUB_LINK,
                        settings::UDPATE_DIR,
                        command_output_formater(&output)
                    ));
                }
            }

            Err(err) => {
                return Err(format!(
                    "Fehler beim Klonen von {} nach {}: {}",
                    settings::GITHUB_LINK,
                    settings::UDPATE_DIR,
                    err
                ));
            }
        }
    } else {
        // Das Directory existiert schon, daher pullen wir einfach den neuen Code
        print_maybe_override!("Altes Update Directory gefunden! Pulle den neuen Code...");
        print_maybe_override!("Info: Dadurch, das das alte Directory noch existiert sollte das Compilen nicht allzu lange dauern.");
        match Command::new("git")
            .arg("reset")
            .arg("--hard")
            .arg("HEAD")
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    return Err(format!(
                        "Fehler beim Resetten der Lokalen changes in {}: {}",
                        settings::UDPATE_DIR,
                        command_output_formater(&output)
                    ));
                }
                command_output_formater(&output);
            }
            Err(err) => {
                return Err(format!(
                    "Fehler beim Resetten der Lokalen changes in {}: {}",
                    settings::UDPATE_DIR,
                    err
                ));
            }
        }
        match Command::new("git")
            .current_dir(settings::UDPATE_DIR)
            .arg("pull")
            .arg("--force")
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    let mut message = String::from("Fehler beim Pullen des neuen Codes: ");
                    message.push_str(command_output_formater(&output).as_str());
                    return Err(message);
                }
            }
            Err(message) => {
                return Err(message.to_string());
            }
        }
    }

    print_maybe_override!("Fertig!");

    if get_current_version() == get_update_version() {
        print_maybe_override!(
            format!("Bereits die neuste Version ({}).", get_current_version()).as_str()
        );
        return Ok(());
    }

    print_maybe_override!(format!(
        "Neue Version gefunden: {} -> {}",
        get_current_version(),
        get_update_version()
    )
    .as_str());
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
    print_maybe_override!(format!(
        "Die neuste Version ({}) ist jetzt installiert.",
        get_update_version()
    )
    .as_str());
    print_maybe_override!("Update erfolgreich abgeschlossen.");
    Ok(())
}
