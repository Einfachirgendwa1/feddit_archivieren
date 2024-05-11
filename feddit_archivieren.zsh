#compdef feddit_archivieren
local -a subcmds
subcmds=('start:Startet den Daemon' 'install:Installiert das Programm' 'kill:Killt den Daemon' 'update:Updated den Daemon' 'update_local:Updated den Daemon anhand des aktuellen Directorys' 'clean:Löscht alle vom Programm erstellten Dateien, außer das Programm selbst' 'info:Zeigt Informationen über den Daemon an' 'logs_static:Zeigt die aktuellen Logs von dem Daemon an' 'checkhealth:Überprüft den "Gesundheitsstatus" des Daemons' 'kill_maybe:Killt den Daemon wenn er läuft')
_describe 'command' subcmds
