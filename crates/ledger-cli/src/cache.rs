use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use zeroize::Zeroizing;

pub struct CacheConfig {
    pub ttl: Duration,
    pub socket_path: PathBuf,
    pub key: String,
}

struct CacheEntry {
    passphrase: Zeroizing<Vec<u8>>,
    stored_at: Instant,
}

pub fn cache_config(path: &Path, ttl_seconds: u64) -> anyhow::Result<Option<CacheConfig>> {
    if ttl_seconds == 0 {
        return Ok(None);
    }
    let socket_path = cache_socket_path()?;
    let key = ledger_hash(path);
    Ok(Some(CacheConfig {
        ttl: Duration::from_secs(ttl_seconds),
        socket_path,
        key,
    }))
}

pub fn cache_get(config: &CacheConfig) -> anyhow::Result<Option<String>> {
    let mut stream = match std::os::unix::net::UnixStream::connect(&config.socket_path) {
        Ok(stream) => stream,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(anyhow::anyhow!("Cache connect failed: {}", err)),
    };
    send_line(&mut stream, &format!("GET {}", config.key))?;
    let response = read_response(&mut stream)?;
    if response.starts_with("PASSPHRASE ") {
        let encoded = response.trim_start_matches("PASSPHRASE ").trim();
        let decoded = STANDARD
            .decode(encoded.as_bytes())
            .map_err(|e| anyhow::anyhow!("Cache decode failed: {}", e))?;
        let passphrase = String::from_utf8(decoded)
            .map_err(|_| anyhow::anyhow!("Cache entry is not valid UTF-8"))?;
        return Ok(Some(passphrase));
    }
    Ok(None)
}

pub fn cache_store(config: &CacheConfig, passphrase: &str) -> anyhow::Result<()> {
    ensure_daemon_running(config)?;
    let mut stream = std::os::unix::net::UnixStream::connect(&config.socket_path)
        .map_err(|e| anyhow::anyhow!("Cache connect failed: {}", e))?;
    let encoded = STANDARD.encode(passphrase.as_bytes());
    send_line(&mut stream, &format!("STORE {} {}", config.key, encoded))?;
    let _ = read_response(&mut stream)?;
    Ok(())
}

pub fn cache_clear(socket_path: &Path) -> anyhow::Result<()> {
    let mut stream = match std::os::unix::net::UnixStream::connect(socket_path) {
        Ok(stream) => stream,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(anyhow::anyhow!("Cache connect failed: {}", err)),
    };
    send_line(&mut stream, "CLEAR")?;
    let _ = read_response(&mut stream)?;
    Ok(())
}

pub fn cache_ping(socket_path: &Path) -> anyhow::Result<bool> {
    let mut stream = match std::os::unix::net::UnixStream::connect(socket_path) {
        Ok(stream) => stream,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(anyhow::anyhow!("Cache connect failed: {}", err)),
    };
    send_line(&mut stream, "PING")?;
    let response = read_response(&mut stream)?;
    Ok(response.trim() == "PONG")
}

pub fn run_cache_daemon(ttl: Duration, socket_path: &Path) -> anyhow::Result<()> {
    let parent = socket_path.parent().ok_or_else(|| {
        anyhow::anyhow!(
            "Cache socket path has no parent directory: {}",
            socket_path.display()
        )
    })?;
    std::fs::create_dir_all(parent).map_err(|e| {
        anyhow::anyhow!(
            "Failed to create cache socket directory {}: {}",
            parent.display(),
            e
        )
    })?;
    if socket_path.exists() {
        let _ = std::fs::remove_file(socket_path);
    }

    let listener = std::os::unix::net::UnixListener::bind(socket_path)
        .map_err(|e| anyhow::anyhow!("Cache bind failed: {}", e))?;
    set_socket_permissions(socket_path)?;
    listener.set_nonblocking(true)?;

    let mut cache: HashMap<String, CacheEntry> = HashMap::new();
    let mut last_activity = Instant::now();

    loop {
        match listener.accept() {
            Ok((mut stream, _addr)) => {
                last_activity = Instant::now();
                let mut buffer = String::new();
                stream.read_to_string(&mut buffer)?;
                let response = handle_request(buffer.trim(), &mut cache, ttl);
                stream.write_all(response.as_bytes())?;
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(err) => return Err(anyhow::anyhow!("Cache accept failed: {}", err)),
        }

        expire_entries(&mut cache, ttl);
        if cache.is_empty() && last_activity.elapsed() >= Duration::from_secs(60) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let _ = std::fs::remove_file(socket_path);
    Ok(())
}

pub fn cache_socket_path() -> anyhow::Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let base = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
        Ok(PathBuf::from(base).join("ledger-cache.sock"));
    }

    #[cfg(not(target_os = "macos"))]
    {
        if let Ok(value) = std::env::var("XDG_RUNTIME_DIR") {
            if !value.trim().is_empty() {
                return Ok(PathBuf::from(value).join("ledger").join("cache.sock"));
            }
        }
        let uid = unsafe { libc::geteuid() };
        Ok(PathBuf::from(format!("/tmp/ledger-{}", uid)).join("cache.sock"))
    }
}

fn handle_request(request: &str, cache: &mut HashMap<String, CacheEntry>, ttl: Duration) -> String {
    let mut parts = request.splitn(3, ' ');
    let command = parts.next().unwrap_or("");
    match command {
        "PING" => "PONG\n".to_string(),
        "CLEAR" => {
            cache.clear();
            "OK\n".to_string()
        }
        "GET" => {
            let key = parts.next().unwrap_or("");
            if let Some(entry) = cache.get(key) {
                if entry.stored_at.elapsed() <= ttl {
                    let encoded = STANDARD.encode(entry.passphrase.as_slice());
                    return format!("PASSPHRASE {}\n", encoded);
                }
            }
            "NOT_FOUND\n".to_string()
        }
        "STORE" => {
            let key = parts.next().unwrap_or("");
            let encoded = parts.next().unwrap_or("");
            match STANDARD.decode(encoded.as_bytes()) {
                Ok(decoded) => {
                    cache.insert(
                        key.to_string(),
                        CacheEntry {
                            passphrase: Zeroizing::new(decoded),
                            stored_at: Instant::now(),
                        },
                    );
                    "OK\n".to_string()
                }
                Err(_) => "ERROR\n".to_string(),
            }
        }
        _ => "ERROR\n".to_string(),
    }
}

fn expire_entries(cache: &mut HashMap<String, CacheEntry>, ttl: Duration) {
    if ttl == Duration::from_secs(0) {
        cache.clear();
        return;
    }
    let expired: Vec<String> = cache
        .iter()
        .filter_map(|(key, entry)| {
            if entry.stored_at.elapsed() > ttl {
                Some(key.clone())
            } else {
                None
            }
        })
        .collect();
    for key in expired {
        cache.remove(&key);
    }
}

fn send_line(stream: &mut std::os::unix::net::UnixStream, line: &str) -> anyhow::Result<()> {
    stream
        .write_all(format!("{}\n", line).as_bytes())
        .map_err(|e| anyhow::anyhow!("Cache write failed: {}", e))
}

fn read_response(stream: &mut std::os::unix::net::UnixStream) -> anyhow::Result<String> {
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| anyhow::anyhow!("Cache read failed: {}", e))?;
    Ok(response.trim().to_string())
}

fn ensure_daemon_running(config: &CacheConfig) -> anyhow::Result<()> {
    if cache_ping(&config.socket_path)? {
        return Ok(());
    }

    let exe = std::env::current_exe().map_err(|e| anyhow::anyhow!("{}", e))?;
    std::process::Command::new(exe)
        .arg("--internal-cache-daemon")
        .arg("--ttl")
        .arg(config.ttl.as_secs().to_string())
        .arg("--socket")
        .arg(&config.socket_path)
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn cache daemon: {}", e))?;

    for _ in 0..20 {
        if cache_ping(&config.socket_path)? {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    Err(anyhow::anyhow!("Cache daemon did not become ready in time"))
}

fn ledger_hash(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let hash = blake3::hash(canonical.to_string_lossy().as_bytes());
    hash.to_hex()[..16].to_string()
}

fn set_socket_permissions(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}
