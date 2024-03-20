//! Der Plan:
//! - Wir überprüfen ob es ein Speicherstand existiert.
//!     - Wenn er nicht existiert, wird er erstellt und mit den Standartwerten gefüllt.
//!     - Wenn er existiert lesen wir ihn und machen da weiter wo wir aufgehört haben.
//!     - Wenn er existiert und ungültig ist, wird fallen wir auf den Standartwert zurück.
//!     - Zudem geben wir eine Fehlermeldung aus.
//! - Dann erstellen wir 2 Threads:
//! - Der erste fetcht Feddit und extrahiert die Daten.
//! - Die Kommunikation erfolgt über einen Arc<Mutex<Speicherstand>>.

//! Funktionen:
//!     read_save
//!     write_save
//!     feddit_thread
//!         -> feddit_fetch
//!         -> feddit_extract
//!         -> feddit_handle
//!             -> feddit_sync
//!
//!     archive_thread
//!         -> archive_thread
//!         -> archive_sync

use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    sync::{Arc, Mutex},
    thread,
};

/// Struct für den Speicherstand.
/// Enthält die URL zu der nächsten feddit-Seite als String,
/// und die aktuellen noch im Channel gespeicherte PostIDs als u128s.
#[derive(Debug)]
struct Speicherstand {
    url: String,
    post_ids: Vec<u128>,
}

impl Speicherstand {
    /// Erstellt einen neuen, leeren Speicherstand.
    fn new() -> Self {
        Speicherstand {
            url: String::new(),
            post_ids: Vec::new(),
        }
    }
}

fn main() {
    let mut speicherstand = Arc::new(Mutex::new(match read_save() {
        Some(save) => save,
        None => {
            if write_save(&Speicherstand::new()).is_none() {
                panic!("Fehler beim Schreiben des Speicherstands.");
            }
            Speicherstand::new()
        }
    }));

    let mut clone = speicherstand.clone();
    let feddit = thread::spawn(move || feddit_thread(&mut clone));
    let archive = thread::spawn(move || archive_thread(&mut speicherstand));

    feddit.join().unwrap();
    archive.join().unwrap();
}

/// Diese Funktion versucht den Speicherstand zu lesen.
/// Bei Erfolg wird der Speicherstand konstruiert und zurückgegeben.
/// Wenn irgendwo ein Fehler aufgetreten ist, wird `None` zurückgegeben.
fn read_save() -> Option<Speicherstand> {
    let file = match File::open("savefile.txt") {
        Ok(file) => file,
        Err(_) => return None,
    };
    let mut line_iterator = BufReader::new(file).lines();
    let url = match line_iterator.next() {
        Some(Ok(url)) => url,
        _ => return None,
    };

    let mut post_ids = vec![];
    for line in line_iterator {
        let line = match line {
            Ok(line) => line,
            Err(_) => return None,
        };
        if line.trim().is_empty() {
            continue;
        }
        post_ids.push(match line.parse::<u128>() {
            Ok(id) => id,
            Err(_) => return None,
        });
    }

    Some(Speicherstand { url, post_ids })
}

/// Diese Funktion schreibt den Speicherstand.
/// * `speicherstand`: Der Speicherstand, der gespeichert werden soll.
fn write_save(speicherstand: &Speicherstand) -> Option<()> {
    let mut file = match File::create("savefile.txt") {
        Ok(file) => file,
        Err(_) => return None,
    };

    if file
        .write_all(format!("{}\n", speicherstand.url).as_bytes())
        .is_err()
    {
        return None;
    }

    for post_id in speicherstand.post_ids.iter() {
        if file.write_all(format!("{}\n", post_id).as_bytes()).is_err() {
            return None;
        }
    }

    Some(())
}

fn feddit_thread(_speicherstand: &mut Arc<Mutex<Speicherstand>>) {
    todo!()
}

fn archive_thread(_speicherstand: &mut Arc<Mutex<Speicherstand>>) {
    todo!()
}
