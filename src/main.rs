//! Der Plan:
//! - Wir überprüfen ob es ein Speicherstand existiert.
//!     - Wenn er nicht existiert, wird er erstellt und mit den Standartwerten gefüllt.
//!     - Wenn er existiert lesen wir ihn und machen da weiter wo wir aufgehört haben.
//!     - Wenn er existiert und ungültig ist, wird fallen wir auf den Standartwert zurück.
//!     - Zudem geben wir eine Fehlermeldung aus.
//! - Dann erstellen wir 2 Threads:
//! - Der erste fetcht Feddit und extrahiert die Daten.
//! - Diese sendet er dann über einen Channel an den zweiten Thread.
//! - Der zweite Thread wartet auf Daten im Channel und fetch dann archive.org.
//! - Alles was in den Channel rein- und rausgeht, wird in der Speicherdatei dokumentiert.

fn main() {
    println!("Hello, world!");
}
