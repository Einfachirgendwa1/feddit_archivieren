#compdef feddit_archivieren
local -a subcmds
subcmds=(
  'start:Startet den Daemon'
  'kill:Killt den Daemon (ohne zu Daten zu sichern)'
  'update:Updated das Programm auf die neuste Version'
  'clean:Löscht alle Dateien vom Programm, bis auf die binarys'
  'info:Zeigt Informationen über den Daemon an'
  'checkhealth:Überprüft den Gesundheitszustand des Daemons'
  'stop:Stoppt den Daemon (sichere Version von kill)'
  'listen:Printet Live was der Daemon ausgibt'
  'uninstall:Deinstalliert das Programm (ruft auch Clean)'
  'install:(DEBUG) Installiert das Programm'
  'update-local:(DEBUG) Updated das Programm mit den Dateien im aktuellen Verzeichnis'
  'kill-maybe:(DEBUG) Killt den Daemon wenn er läuft'
  'logs-static:(DEBUG) Zeigt die Logs des Daemons an'
)
_describe 'command' subcmds
