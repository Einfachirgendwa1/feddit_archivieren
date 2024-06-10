#![allow(dead_code)]

pub const GIT_BRANCH: &'static str = "feat/feddit";
pub const DAEMON_PATH: &'static str = "/usr/bin/feddit_archive_daemon";
pub const CLIENT_PATH: &'static str = "/usr/bin/feddit_archivieren";
pub const RUN_DIR: &'static str = "/run/feddit_archivieren";
pub const PID_FILE: &'static str = "/run/feddit_archivieren/daemon.pid";
pub const ERR_FILE: &'static str = "/run/feddit_archivieren/daemon.err";
pub const OUT_FILE: &'static str = "/run/feddit_archivieren/daemon.out";
pub const URL_FILE: &'static str = "/run/feddit_archivieren/url.txt";
pub const UPDATE_LOG_FILE: &'static str = "/run/feddit_archivieren/update_log.txt";
pub const POST_FILE: &'static str = "/run/feddit_archivieren/posts.txt";
pub const SOCKET_FILE: &'static str = "/run/feddit_archivieren/daemon.sck";
pub const UDPATE_DIR: &'static str = "/var/tmp/feddit_archivieren";
pub const UDPATE_CACHE_DIR: &'static str = "/var/tmp/feddit_archivieren_cache";
pub const GITHUB_LINK: &'static str = "https://github.com/Einfachirgendwa1/feddit_archivieren";
pub const FEDDIT_URL: &'static str = "https://lemmy.ml/api/v3/post/list";

pub const TCP_BUFFER_SIZE: usize = 1024;
pub const UPDATE_FETCH_DELAY: std::time::Duration = std::time::Duration::from_secs(120);
pub const FEDDIT_FETCH_DELAY: std::time::Duration = std::time::Duration::from_secs(10);
