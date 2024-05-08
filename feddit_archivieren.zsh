#compdef feddit_archivieren
local -a subcmds
subcmds=('start:Startet den Daemon' 'install:Installiert das Programm' 'kill:Killt den Daemon' 'update:TODO:Updated den Daemon' 'update_local:TODO:Updated den Daemon anhand des aktuellen Directorys')
_describe 'command' subcmds
