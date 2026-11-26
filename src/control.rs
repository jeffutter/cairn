//! The control socket: how a CLI finds the running `cairn serve` (ADR 0009).
//!
//! `serve` listens on a unix socket whose path is derived from the database
//! path, so the server and any CLI using the same database agree on it with no
//! configuration. The protocol is one request line and one response line per
//! connection. The only request is `status`; the response is a JSON
//! [`Status`], or `{"error": "..."}` for anything else.
//!
//! Ownership of the socket is an exclusive lock on a file beside it, held for
//! the server's lifetime. The kernel drops the lock when the process dies, so a
//! socket file left by a killed server is safe to replace.

use std::ffi::OsString;
use std::fs::{File, TryLockError};
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::os::unix::net::{UnixListener as StdUnixListener, UnixStream as StdUnixStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

use crate::error::Error;

const STATUS_REQUEST: &str = "status";
/// Bounds on what either side will read, and how long it will wait.
const MAX_REQUEST_BYTES: u64 = 64;
const MAX_RESPONSE_BYTES: u64 = 4096;
const TIMEOUT: Duration = Duration::from_secs(2);

/// Where a running server can be reached.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// Base URL of the review UI and API
    pub ui: String,
    /// Base URL of the untrusted-content origin
    pub content: String,
    pub ui_port: u16,
    pub content_port: u16,
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = OsString::from(path);
    s.push(suffix);
    PathBuf::from(s)
}

/// The socket of the server for the database at `db`: `<db>.sock`.
pub fn socket_path(db: &Path) -> PathBuf {
    with_suffix(db, ".sock")
}

/// The exclusive right to serve one database's control socket. Dropping it
/// removes the socket file, then releases the lock.
pub(crate) struct Claim {
    socket: PathBuf,
    _lock: File,
}

impl Claim {
    /// Take the lock, or fail naming the server that already holds it.
    pub(crate) fn acquire(db: &Path) -> Result<Claim, Error> {
        let socket = socket_path(db);
        let lock_path = with_suffix(&socket, ".lock");
        let io = |e: std::io::Error| {
            Error::Unexpected(format!("cannot lock {}: {e}", lock_path.display()))
        };
        let lock = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)
            .map_err(io)?;
        match lock.try_lock() {
            Ok(()) => Ok(Claim {
                socket,
                _lock: lock,
            }),
            Err(TryLockError::WouldBlock) => {
                let at = match query(db) {
                    Ok(Some(s)) => format!(" at {}", s.ui),
                    _ => String::new(),
                };
                Err(Error::Unexpected(format!(
                    "a cairn server{at} is already serving {}",
                    db.display()
                )))
            }
            Err(TryLockError::Error(e)) => Err(io(e)),
        }
    }

    /// Bind the socket, readable and writable by this user only.
    pub(crate) fn listen(&self) -> Result<StdUnixListener, Error> {
        let path = &self.socket;
        let io = |e: std::io::Error| {
            Error::Unexpected(format!("cannot listen on {}: {e}", path.display()))
        };
        // Holding the lock means any socket here is a dead server's. Anything
        // else at this path is not ours to delete; bind then fails on it.
        match std::fs::symlink_metadata(path) {
            Ok(m) if m.file_type().is_socket() => std::fs::remove_file(path).map_err(io)?,
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(io(e)),
        }
        let l = StdUnixListener::bind(path).map_err(io)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(io)?;
        l.set_nonblocking(true).map_err(io)?;
        Ok(l)
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        if std::fs::symlink_metadata(&self.socket).is_ok_and(|m| m.file_type().is_socket()) {
            let _ = std::fs::remove_file(&self.socket);
        }
    }
}

/// Answer requests on `listener` until the future is dropped.
pub(crate) async fn answer(listener: UnixListener, status: Status) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let status = status.clone();
                tokio::spawn(async move {
                    // A client that stalls or hangs up only loses its answer.
                    let _ = tokio::time::timeout(TIMEOUT, reply(stream, &status)).await;
                });
            }
            // Accept fails transiently (e.g. out of descriptors); do not spin.
            Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
}

async fn reply(stream: UnixStream, status: &Status) -> std::io::Result<()> {
    let (read, mut write) = stream.into_split();
    let mut request = String::new();
    tokio::io::BufReader::new(read.take(MAX_REQUEST_BYTES))
        .read_line(&mut request)
        .await?;
    let mut response = match request.trim_end() {
        STATUS_REQUEST => serde_json::to_string(status).expect("status serializes"),
        _ => r#"{"error":"unknown request"}"#.to_string(),
    };
    response.push('\n');
    write.write_all(response.as_bytes()).await?;
    write.shutdown().await
}

/// Ask the server for the database at `db` where it is. `None` means no server
/// is running; a server that cannot be asked or answers badly is an error.
pub fn query(db: &Path) -> Result<Option<Status>, Error> {
    let path = socket_path(db);
    let fail = |e: &dyn std::fmt::Display| {
        Error::Unexpected(format!(
            "cannot ask the cairn server at {}: {e}",
            path.display()
        ))
    };
    let mut stream = match StdUnixStream::connect(&path) {
        Ok(s) => s,
        // No socket, a dead server's socket, or a path too long to have ever
        // been bound.
        Err(e)
            if matches!(
                e.kind(),
                ErrorKind::NotFound
                    | ErrorKind::NotADirectory
                    | ErrorKind::ConnectionRefused
                    | ErrorKind::InvalidInput
            ) =>
        {
            return Ok(None);
        }
        Err(e) => return Err(fail(&e)),
    };
    let mut response = String::new();
    stream
        .set_read_timeout(Some(TIMEOUT))
        .and_then(|()| stream.set_write_timeout(Some(TIMEOUT)))
        .and_then(|()| writeln!(stream, "{STATUS_REQUEST}"))
        .and_then(|()| BufReader::new(stream.take(MAX_RESPONSE_BYTES)).read_line(&mut response))
        .map_err(|e| fail(&e))?;
    serde_json::from_str(&response)
        .map(Some)
        .map_err(|e| fail(&format!("bad response: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status() -> Status {
        Status {
            ui: "http://127.0.0.1:1".into(),
            content: "http://127.0.0.1:2".into(),
            ui_port: 1,
            content_port: 2,
        }
    }

    #[test]
    fn socket_sits_beside_the_database() {
        assert_eq!(
            socket_path(Path::new("/d/cairn.db")),
            PathBuf::from("/d/cairn.db.sock")
        );
    }

    #[test]
    fn no_socket_and_a_stale_socket_mean_no_server() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("c.db");
        assert_eq!(query(&db).unwrap(), None);
        drop(StdUnixListener::bind(socket_path(&db)).unwrap());
        assert!(socket_path(&db).exists());
        assert_eq!(query(&db).unwrap(), None);
        // A new server replaces it, and removes its own on the way out.
        let claim = Claim::acquire(&db).unwrap();
        claim.listen().unwrap();
        drop(claim);
        assert!(!socket_path(&db).exists());
    }

    #[test]
    fn a_second_claim_is_refused_until_the_first_drops() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("c.db");
        let first = Claim::acquire(&db).unwrap();
        let Err(e) = Claim::acquire(&db) else {
            panic!("second claim succeeded");
        };
        assert!(e.to_string().contains("already serving"), "{e}");
        drop(first);
        Claim::acquire(&db).unwrap();
    }

    #[test]
    fn a_regular_file_at_the_socket_path_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("c.db");
        std::fs::write(socket_path(&db), "keep").unwrap();
        let claim = Claim::acquire(&db).unwrap();
        assert!(claim.listen().is_err());
        drop(claim);
        assert_eq!(std::fs::read_to_string(socket_path(&db)).unwrap(), "keep");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn answers_status_and_rejects_other_requests() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("c.db");
        let claim = Claim::acquire(&db).unwrap();
        let listener = UnixListener::from_std(claim.listen().unwrap()).unwrap();
        tokio::spawn(answer(listener, status()));
        let q = db.clone();
        let got = tokio::task::spawn_blocking(move || query(&q))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got, Some(status()));

        let mut s = UnixStream::connect(socket_path(&db)).await.unwrap();
        s.write_all(b"nonsense\n").await.unwrap();
        let mut r = String::new();
        s.read_to_string(&mut r).await.unwrap();
        assert_eq!(r, "{\"error\":\"unknown request\"}\n");
    }
}
