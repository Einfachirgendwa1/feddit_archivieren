#![allow(dead_code)]
#![cfg_attr(rustfmt, rustfmt::skip)]

pub const DAEMON_PATH: &'static str = "/usr/bin/feddit_archive_daemon";
pub const CLIENT_PATH: &'static str = "/usr/bin/feddit_archivieren";
pub const RUN_DIR:     &'static str = "/run/feddit_archivieren";
pub const PID_FILE:    &'static str = "/run/feddit_archivieren/daemon.pid";
pub const ERR_FILE:    &'static str = "/run/feddit_archivieren/daemon.err";
pub const OUT_FILE:    &'static str = "/run/feddit_archivieren/daemon.out";
pub const URL_FILE:    &'static str = "/run/feddit_archivieren/url.txt";
pub const POST_FILE:   &'static str = "/run/feddit_archivieren/posts.txt";
pub const SOCKET_FILE: &'static str = "/run/feddit_archivieren/daemon.sck";
pub const UDPATE_DIR:  &'static str = "/var/tmp/feddit_archivieren";
pub const GITHUB_LINK: &'static str = "https://github.com/Einfachirgendwa1/feddit_archivieren";
pub const FEDDIT_LINK: &'static str = "https://feddit.de/?dataType=Post&listingType=Local&page=1&sort=New";

pub const TCP_BUFFER_SIZE: usize = 1024;
pub const UPDATE_FETCH_DELAY: std::time::Duration = std::time::Duration::from_secs(60 * 15);
