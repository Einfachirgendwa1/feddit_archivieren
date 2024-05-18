#![allow(dead_code)]
pub const DAEMON_PATH: &'static str = "/usr/bin/feddit_archive_daemon";
pub const CLIENT_PATH: &'static str = "/usr/bin/feddit_archivieren";
pub const RUN_DIR: &'static str = "/run/feddit_archivieren";
pub const PID_FILE: &'static str = "/run/feddit_archivieren/daemon.pid";
pub const ERR_FILE: &'static str = "/run/feddit_archivieren/daemon.err";
pub const OUT_FILE: &'static str = "/run/feddit_archivieren/daemon.out";
pub const SOCKET_FILE: &'static str = "/run/feddit_archivieren/daemon.sck";
pub const UDPATE_DIR: &'static str = "/var/tmp/feddit_archivieren";
pub const GITHUB_LINK: &'static str = "https://github.com/Einfachirgendwa1/feddit_archivieren";
pub const TCP_BUFFER_SIZE: usize = 1024;
pub const FEDDIT_LINK: &'static str =
    "https://feddit.de/?dataType=Post&listingType=Local&page=1&sort=New";
