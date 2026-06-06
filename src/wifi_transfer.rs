//! Explicitly activated Wi-Fi SD-card transfer web portal.
//!
//! The portal is intentionally dormant after boot.  The user must enter
//! Settings > Network and select Start Wi-Fi Transfer.  The target-specific
//! HTTP server owns its own ESP-IDF task while the main loop retains display,
//! routing and sleep ownership.

use std::path::{Component, Path, PathBuf};

/// Portal root.  Browser requests may never escape this subtree.
pub const WIFI_TRANSFER_ROOT: &str = "/sdcard/RUSTMIX";
/// The ESP-IDF HTTP server task exists only while transfer mode is enabled.
pub const WIFI_TRANSFER_SERVER_STACK_BYTES: usize = 24 * 1024;
/// Upload and download bodies stream through one bounded scratch buffer.
pub const WIFI_TRANSFER_STREAM_CHUNK_BYTES: usize = 4 * 1024;
/// Keep accidental huge uploads bounded for the first portal slice.
pub const WIFI_TRANSFER_MAX_UPLOAD_BYTES: usize = 64 * 1024 * 1024;
/// Bound decoded browser paths before touching removable storage.
pub const WIFI_TRANSFER_MAX_PATH_BYTES: usize = 128;
/// Stop a forgotten transfer portal after ten minutes without HTTP traffic.
pub const WIFI_TRANSFER_INACTIVITY_SECONDS: u64 = 10 * 60;
/// The first slice uses the conventional LAN HTTP port.
pub const WIFI_TRANSFER_HTTP_PORT: u16 = 80;
/// One authenticated browser session code is shown on the e-paper screen.
pub const WIFI_TRANSFER_CODE_DIGITS: usize = 6;
/// Bound one directory listing to protect the HTTP task heap.
pub const WIFI_TRANSFER_MAX_DIRECTORY_ROWS: usize = 256;

/// User request handed from the hardware-independent UI into `main.rs`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WifiTransferUiRequest {
    Start,
    Stop,
}

/// Compact UI-facing lifecycle state.  This snapshot never contains a Wi-Fi
/// password and remains safe to render or log.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WifiTransferState {
    #[default]
    Off,
    Starting,
    Ready,
    Failed,
}

impl WifiTransferState {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::Starting => "STARTING",
            Self::Ready => "READY",
            Self::Failed => "FAILED",
        }
    }
}

/// Small main-loop snapshot updated when the server starts, stops or handles a
/// completed filesystem request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WifiTransferSnapshot {
    pub state: WifiTransferState,
    pub url: Option<String>,
    pub code: Option<String>,
    pub last_action: String,
    pub last_bytes: usize,
    pub error: Option<String>,
}

impl Default for WifiTransferSnapshot {
    fn default() -> Self {
        Self {
            state: WifiTransferState::Off,
            url: None,
            code: None,
            last_action: "Portal is off".into(),
            last_bytes: 0,
            error: None,
        }
    }
}

impl WifiTransferSnapshot {
    #[must_use]
    pub fn starting() -> Self {
        Self {
            state: WifiTransferState::Starting,
            last_action: "Starting LAN portal".into(),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            state: WifiTransferState::Failed,
            last_action: "Portal start failed".into(),
            error: Some(error.into()),
            ..Self::default()
        }
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(
            self.state,
            WifiTransferState::Starting | WifiTransferState::Ready
        )
    }

    #[must_use]
    pub fn url_label(&self) -> &str {
        self.url.as_deref().unwrap_or("--")
    }

    #[must_use]
    pub fn code_label(&self) -> &str {
        self.code.as_deref().unwrap_or("------")
    }
}

/// Reject empty, traversal, absolute and overlong relative portal paths.
/// Returned paths always stay beneath `/sdcard/RUSTMIX`.
pub fn resolve_portal_path(relative: &str) -> Result<PathBuf, &'static str> {
    let decoded = percent_decode(relative)?;
    if decoded.len() > WIFI_TRANSFER_MAX_PATH_BYTES {
        return Err("path exceeds portal limit");
    }
    let trimmed = decoded.trim_start_matches('/');
    let relative_path = Path::new(trimmed);
    let mut safe = PathBuf::from(WIFI_TRANSFER_ROOT);
    for component in relative_path.components() {
        match component {
            Component::Normal(name) => {
                let name = name.to_str().ok_or("path is not UTF-8")?;
                if !is_fat83_component(name) {
                    return Err("use FAT 8.3-safe names");
                }
                safe.push(name);
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err("path traversal is blocked")
            }
        }
    }
    Ok(safe)
}

/// Configuration files stay hidden and cannot be modified by the initial LAN
/// portal.  This prevents accidental credential disclosure or live config
/// replacement while services are running.
#[must_use]
pub fn is_protected_portal_path(relative: &str) -> bool {
    let upper = relative.trim_start_matches('/').to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "WIFI.TXT"
            | "ALARMS.TXT"
            | "DISPLAY.TXT"
            | "WEATHER.TXT"
            | "VOICE/META.TXT"
            | "VOICE/SETTINGS.TXT"
            | "APPS/CALENDAR/EVENTS.TMP"
            | "APPS/CALENDAR/EVENTS.BAK"
    )
}

/// Folder names are at most eight uppercase-safe characters.  Files are 8.3.
#[must_use]
pub fn is_fat83_component(component: &str) -> bool {
    if component.is_empty() || component == "." || component == ".." {
        return false;
    }
    let mut parts = component.split('.');
    let stem = parts.next().unwrap_or_default();
    let extension = parts.next();
    if parts.next().is_some() || stem.is_empty() || stem.len() > 8 {
        return false;
    }
    if extension.is_some_and(|value| value.is_empty() || value.len() > 3) {
        return false;
    }
    component
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'~'))
}

/// Tiny query parser used by the portal API.  The firmware intentionally avoids
/// allocating a generic web framework.
#[must_use]
pub fn query_value(uri: &str, key: &str) -> Option<String> {
    let query = uri.split_once('?')?.1;
    query.split('&').find_map(|part| {
        let (candidate, value) = part.split_once('=')?;
        (candidate == key)
            .then(|| percent_decode(value).ok())
            .flatten()
    })
}

fn percent_decode(value: &str) -> Result<String, &'static str> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let high = hex(bytes[index + 1]).ok_or("invalid percent escape")?;
                let low = hex(bytes[index + 2]).ok_or("invalid percent escape")?;
                output.push((high << 4) | low);
                index += 3;
            }
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(output).map_err(|_| "path is not UTF-8")
}

const fn hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(target_os = "espidf")]
pub mod espidf {
    use std::{
        fs::{self, File},
        io::{Read as StdRead, Write as StdWrite},
        path::Path,
        sync::{Arc, Mutex},
        time::{Duration, Instant},
    };

    use anyhow::{anyhow, bail, Context, Result};
    use embedded_svc::{http::Method, io::Write as _};
    use esp_idf_svc::http::server::{Configuration, EspHttpServer};
    use log::info;

    use super::{
        is_protected_portal_path, query_value, resolve_portal_path, WifiTransferSnapshot,
        WifiTransferState, WIFI_TRANSFER_HTTP_PORT, WIFI_TRANSFER_INACTIVITY_SECONDS,
        WIFI_TRANSFER_MAX_DIRECTORY_ROWS, WIFI_TRANSFER_MAX_UPLOAD_BYTES, WIFI_TRANSFER_ROOT,
        WIFI_TRANSFER_SERVER_STACK_BYTES, WIFI_TRANSFER_STREAM_CHUNK_BYTES,
    };

    const PORTAL_HTML: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Rustmix-Wave Transfer</title><style>
body{font-family:sans-serif;max-width:900px;margin:2rem auto;padding:0 1rem}button,input{padding:.65rem;margin:.2rem}pre{white-space:pre-wrap;background:#f4f4f4;padding:1rem}table{width:100%;border-collapse:collapse}td,th{border-bottom:1px solid #ddd;padding:.5rem;text-align:left}
</style></head><body><h1>Rustmix-Wave Wi-Fi Transfer</h1>
<p>LAN-only SD portal rooted at <code>/RUSTMIX</code>. Use FAT 8.3-safe names.</p>
<label>Session code <input id="code" maxlength="6"><button onclick="saveCode()">Unlock</button></label>
<p><button onclick="loadList('/')">Home</button><button onclick="loadList(current)">Refresh</button></p>
<p>Path: <code id="path">/</code></p><div id="list"></div>
<h2>Upload</h2><input id="file" type="file"><input id="name" placeholder="BOOK0001.TXT"><button onclick="upload()">Upload</button>
<h2>Folder</h2><input id="folder" placeholder="NEWFOLD"><button onclick="mkdir()">Create folder</button>
<pre id="status">Enter the six-digit code displayed on the device.</pre>
<script>
let current='/'; const status=t=>document.getElementById('status').textContent=t;
const code=()=>localStorage.rustmixCode||document.getElementById('code').value;
function saveCode(){localStorage.rustmixCode=document.getElementById('code').value;loadList(current)}
const enc=s=>encodeURIComponent(s); const join=n=>(current==='/'?'/':current+'/')+n;
async function api(url,opt){let r=await fetch(url+(url.includes('?')?'&':'?')+'code='+enc(code()),opt);let t=await r.text();if(!r.ok)throw Error(t);return t}
async function loadList(path){try{current=path;document.getElementById('path').textContent=path;let rows=JSON.parse(await api('/api/list?path='+enc(path)));let h='<table><tr><th>Name</th><th>Type</th><th>Size</th><th>Actions</th></tr>';if(path!='/')h+='<tr><td><button onclick="up()">..</button></td><td>folder</td><td></td><td></td></tr>';for(let e of rows){let p=join(e.name);h+='<tr><td>'+e.name+'</td><td>'+e.kind+'</td><td>'+e.size+'</td><td>'+(e.kind==='folder'?'<button onclick="loadList(\''+p+'\')">Open</button>':'<a href="/api/download?code='+enc(code())+'&path='+enc(p)+'">Download</a>')+' <button onclick="renamePath(\''+p+'\')">Rename</button> <button onclick="del(\''+p+'\')">Delete</button></td></tr>'}h+='</table>';document.getElementById('list').innerHTML=h;status('Ready')}catch(e){status(e.message)}}
function up(){let p=current.split('/').filter(Boolean);p.pop();loadList('/'+p.join('/'))}
async function upload(){try{let f=document.getElementById('file').files[0];let n=document.getElementById('name').value||f.name;await api('/api/upload?path='+enc(join(n)),{method:'POST',body:f});status('Upload complete');loadList(current)}catch(e){status(e.message)}}
async function mkdir(){try{await api('/api/mkdir?path='+enc(join(document.getElementById('folder').value)),{method:'POST'});status('Folder created');loadList(current)}catch(e){status(e.message)}}
async function renamePath(p){try{let n=prompt('New FAT 8.3-safe name');if(!n)return;let parent=p.substring(0,p.lastIndexOf('/'))||'/';let to=(parent==='/'?'/':parent+'/')+n;await api('/api/rename?from='+enc(p)+'&to='+enc(to),{method:'POST'});status('Renamed');loadList(current)}catch(e){status(e.message)}}
async function del(p){try{await api('/api/delete?path='+enc(p),{method:'POST'});status('Deleted');loadList(current)}catch(e){status(e.message)}}
loadList('/');
</script></body></html>"#;

    #[derive(Debug)]
    struct SharedStatus {
        snapshot: WifiTransferSnapshot,
        last_activity: Instant,
    }

    impl SharedStatus {
        fn new(url: String, code: String) -> Self {
            Self {
                snapshot: WifiTransferSnapshot {
                    state: WifiTransferState::Ready,
                    url: Some(url),
                    code: Some(code),
                    last_action: "Portal ready".into(),
                    last_bytes: 0,
                    error: None,
                },
                last_activity: Instant::now(),
            }
        }

        fn touch(&mut self, action: impl Into<String>, bytes: usize) {
            self.snapshot.last_action = action.into();
            self.snapshot.last_bytes = bytes;
            self.snapshot.error = None;
            self.last_activity = Instant::now();
        }
    }

    /// RAII wrapper.  Constructing starts ESP-IDF's dedicated HTTP task;
    /// dropping stops that task and frees its resources.
    pub struct WifiTransferServer {
        _server: EspHttpServer<'static>,
        shared: Arc<Mutex<SharedStatus>>,
    }

    impl WifiTransferServer {
        pub fn start(ipv4: &str, code: String) -> Result<Self> {
            let url = format!("http://{ipv4}/");
            let shared = Arc::new(Mutex::new(SharedStatus::new(url.clone(), code.clone())));
            let mut server = EspHttpServer::new(&Configuration {
                http_port: WIFI_TRANSFER_HTTP_PORT,
                stack_size: WIFI_TRANSFER_SERVER_STACK_BYTES,
                max_open_sockets: 2,
                max_sessions: 2,
                max_uri_handlers: 10,
                session_timeout: Duration::from_secs(60),
                ..Default::default()
            })?;

            server.fn_handler("/", Method::Get, move |request| {
                request
                    .into_ok_response()?
                    .write_all(PORTAL_HTML.as_bytes())?;
                Ok::<(), anyhow::Error>(())
            })?;

            let list_shared = Arc::clone(&shared);
            let list_code = code.clone();
            server.fn_handler("/api/list", Method::Get, move |request| {
                authenticate(request.uri(), &list_code)?;
                let relative = query_value(request.uri(), "path").unwrap_or_else(|| "/".into());
                let body = list_directory_json(&relative)?;
                lock(&list_shared).touch(format!("Listed {relative}"), body.len());
                request.into_ok_response()?.write_all(body.as_bytes())?;
                Ok::<(), anyhow::Error>(())
            })?;

            let download_shared = Arc::clone(&shared);
            let download_code = code.clone();
            server.fn_handler("/api/download", Method::Get, move |request| {
                authenticate(request.uri(), &download_code)?;
                let relative = required_query(request.uri(), "path")?;
                reject_protected(&relative)?;
                let path = resolve_portal_path(&relative).map_err(|error| anyhow!(error))?;
                let mut file = File::open(&path).with_context(|| format!("open {}", path.display()))?;
                let mut response = request.into_ok_response()?;
                let mut buffer = [0_u8; WIFI_TRANSFER_STREAM_CHUNK_BYTES];
                let mut total = 0;
                loop {
                    let read = StdRead::read(&mut file, &mut buffer)?;
                    if read == 0 { break; }
                    response.write_all(&buffer[..read])?;
                    total += read;
                }
                lock(&download_shared).touch(format!("Downloaded {relative}"), total);
                info!("rustmix-wave=wifi-transfer-request method=GET route=download path={relative} bytes={total} status=completed");
                Ok::<(), anyhow::Error>(())
            })?;

            let upload_shared = Arc::clone(&shared);
            let upload_code = code.clone();
            server.fn_handler("/api/upload", Method::Post, move |mut request| {
                authenticate(request.uri(), &upload_code)?;
                let relative = required_query(request.uri(), "path")?;
                reject_protected(&relative)?;
                let path = resolve_portal_path(&relative).map_err(|error| anyhow!(error))?;
                let temporary = temporary_path(&path)?;
                if temporary.exists() {
                    fs::remove_file(&temporary)?;
                }
                let transfer_result = (|| -> Result<usize> {
                    let mut file = File::create(&temporary)
                        .with_context(|| format!("create {}", temporary.display()))?;
                    let mut buffer = [0_u8; WIFI_TRANSFER_STREAM_CHUNK_BYTES];
                    let mut total = 0;
                    loop {
                        let read = request.read(&mut buffer)?;
                        if read == 0 { break; }
                        total += read;
                        if total > WIFI_TRANSFER_MAX_UPLOAD_BYTES {
                            bail!("upload exceeds 64 MiB limit");
                        }
                        StdWrite::write_all(&mut file, &buffer[..read])?;
                    }
                    StdWrite::flush(&mut file)?;
                    Ok(total)
                })();
                let total = match transfer_result {
                    Ok(total) => total,
                    Err(error) => {
                        let _ = fs::remove_file(&temporary);
                        return Err(error);
                    }
                };
                commit_atomic_upload(&temporary, &path)?;
                lock(&upload_shared).touch(format!("Uploaded {relative}"), total);
                info!("rustmix-wave=wifi-transfer-request method=POST route=upload path={relative} bytes={total} status=completed");
                request.into_ok_response()?.write_all(b"uploaded")?;
                Ok::<(), anyhow::Error>(())
            })?;

            let delete_shared = Arc::clone(&shared);
            let delete_code = code.clone();
            server.fn_handler("/api/delete", Method::Post, move |request| {
                authenticate(request.uri(), &delete_code)?;
                let relative = required_query(request.uri(), "path")?;
                reject_protected(&relative)?;
                let path = resolve_portal_path(&relative).map_err(|error| anyhow!(error))?;
                if path.is_dir() { fs::remove_dir(&path)?; } else { fs::remove_file(&path)?; }
                lock(&delete_shared).touch(format!("Deleted {relative}"), 0);
                info!("rustmix-wave=wifi-transfer-request method=POST route=delete path={relative} status=completed");
                request.into_ok_response()?.write_all(b"deleted")?;
                Ok::<(), anyhow::Error>(())
            })?;

            let mkdir_shared = Arc::clone(&shared);
            let mkdir_code = code.clone();
            server.fn_handler("/api/mkdir", Method::Post, move |request| {
                authenticate(request.uri(), &mkdir_code)?;
                let relative = required_query(request.uri(), "path")?;
                reject_protected(&relative)?;
                let path = resolve_portal_path(&relative).map_err(|error| anyhow!(error))?;
                fs::create_dir(&path)?;
                lock(&mkdir_shared).touch(format!("Created {relative}"), 0);
                info!("rustmix-wave=wifi-transfer-request method=POST route=mkdir path={relative} status=completed");
                request.into_ok_response()?.write_all(b"created")?;
                Ok::<(), anyhow::Error>(())
            })?;

            let rename_shared = Arc::clone(&shared);
            let rename_code = code.clone();
            server.fn_handler("/api/rename", Method::Post, move |request| {
                authenticate(request.uri(), &rename_code)?;
                let from = required_query(request.uri(), "from")?;
                let to = required_query(request.uri(), "to")?;
                reject_protected(&from)?;
                reject_protected(&to)?;
                let source = resolve_portal_path(&from).map_err(|error| anyhow!(error))?;
                let destination = resolve_portal_path(&to).map_err(|error| anyhow!(error))?;
                fs::rename(&source, &destination)?;
                lock(&rename_shared).touch(format!("Renamed {from}"), 0);
                info!("rustmix-wave=wifi-transfer-request method=POST route=rename from={from} to={to} status=completed");
                request.into_ok_response()?.write_all(b"renamed")?;
                Ok::<(), anyhow::Error>(())
            })?;

            let status_shared = Arc::clone(&shared);
            let status_code = code.clone();
            server.fn_handler("/api/status", Method::Get, move |request| {
                authenticate(request.uri(), &status_code)?;
                let snapshot = lock(&status_shared).snapshot.clone();
                let body = format!(
                    "{{\"state\":\"{}\",\"last_action\":\"{}\",\"last_bytes\":{}}}",
                    snapshot.state.label(),
                    json_escape(&snapshot.last_action),
                    snapshot.last_bytes
                );
                request.into_ok_response()?.write_all(body.as_bytes())?;
                Ok::<(), anyhow::Error>(())
            })?;

            info!("rustmix-wave=wifi-transfer-server status=ready url={url} root={WIFI_TRANSFER_ROOT} stack-bytes={WIFI_TRANSFER_SERVER_STACK_BYTES}");
            Ok(Self {
                _server: server,
                shared,
            })
        }

        #[must_use]
        pub fn snapshot(&self) -> WifiTransferSnapshot {
            lock(&self.shared).snapshot.clone()
        }

        #[must_use]
        pub fn is_expired(&self) -> bool {
            lock(&self.shared).last_activity.elapsed()
                >= Duration::from_secs(WIFI_TRANSFER_INACTIVITY_SECONDS)
        }
    }

    impl Drop for WifiTransferServer {
        fn drop(&mut self) {
            info!("rustmix-wave=wifi-transfer-server status=stopped");
        }
    }

    fn lock(shared: &Arc<Mutex<SharedStatus>>) -> std::sync::MutexGuard<'_, SharedStatus> {
        shared
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn authenticate(uri: &str, expected: &str) -> Result<()> {
        if query_value(uri, "code").as_deref() == Some(expected) {
            Ok(())
        } else {
            bail!("session code required")
        }
    }

    fn required_query(uri: &str, key: &str) -> Result<String> {
        query_value(uri, key).ok_or_else(|| anyhow!("missing query parameter: {key}"))
    }

    fn reject_protected(relative: &str) -> Result<()> {
        if is_protected_portal_path(relative) {
            bail!("protected configuration file")
        }
        Ok(())
    }

    fn sibling_path(path: &Path, extension: &str) -> Result<std::path::PathBuf> {
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| anyhow!("filename required"))?;
        let parent = path
            .parent()
            .ok_or_else(|| anyhow!("parent folder required"))?;
        Ok(parent.join(format!("{stem}.{extension}")))
    }

    fn temporary_path(path: &Path) -> Result<std::path::PathBuf> {
        sibling_path(path, "TMP")
    }

    fn commit_atomic_upload(temporary: &Path, destination: &Path) -> Result<()> {
        let backup = sibling_path(destination, "BAK")?;
        if backup.exists() {
            fs::remove_file(&backup)?;
        }
        let had_destination = destination.exists();
        if had_destination {
            fs::rename(destination, &backup)?;
        }
        if let Err(error) = fs::rename(temporary, destination) {
            if had_destination {
                let _ = fs::rename(&backup, destination);
            }
            let _ = fs::remove_file(temporary);
            return Err(error.into());
        }
        if had_destination {
            fs::remove_file(&backup)?;
        }
        Ok(())
    }

    fn list_directory_json(relative: &str) -> Result<String> {
        let path = resolve_portal_path(relative).map_err(|error| anyhow!(error))?;
        let mut entries = Vec::new();
        for entry in fs::read_dir(path)?.take(WIFI_TRANSFER_MAX_DIRECTORY_ROWS) {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let child_relative = format!("{}/{}", relative.trim_end_matches('/'), name);
            if is_protected_portal_path(&child_relative) || !super::is_fat83_component(&name) {
                continue;
            }
            let metadata = entry.metadata()?;
            entries.push((name, metadata.is_dir(), metadata.len()));
        }
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        let mut json = String::from("[");
        for (index, (name, directory, size)) in entries.into_iter().enumerate() {
            if index > 0 {
                json.push(',');
            }
            let kind = if directory { "folder" } else { "file" };
            json.push_str(&format!(
                r#"{{"name":"{}","kind":"{kind}","size":{size}}}"#,
                json_escape(&name)
            ));
        }
        json.push(']');
        Ok(json)
    }

    fn json_escape(value: &str) -> String {
        value.replace('\\', "\\\\").replace('"', "\\\"")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        is_fat83_component, is_protected_portal_path, query_value, resolve_portal_path,
        WifiTransferSnapshot, WifiTransferState,
    };

    #[test]
    fn portal_is_off_until_the_user_explicitly_starts_it() {
        let snapshot = WifiTransferSnapshot::default();
        assert_eq!(snapshot.state, WifiTransferState::Off);
        assert!(!snapshot.is_active());
    }

    #[test]
    fn portal_paths_are_confined_and_fat83_safe() {
        assert!(resolve_portal_path("/BOOKS/POIROT01.EPU").is_ok());
        assert!(resolve_portal_path("../WIFI.TXT").is_err());
        assert!(resolve_portal_path("/BOOKS/long-file-name.txt").is_err());
        assert!(is_fat83_component("MAIN.LUA"));
        assert!(is_fat83_component("SUDOKU"));
        assert!(!is_fat83_component("NOT FAT SAFE.TXT"));
    }

    #[test]
    fn configuration_files_are_protected() {
        assert!(is_protected_portal_path("/WIFI.TXT"));
        assert!(is_protected_portal_path("ALARMS.TXT"));
        assert!(is_protected_portal_path("/VOICE/META.TXT"));
        assert!(is_protected_portal_path("VOICE/SETTINGS.TXT"));
        assert!(is_protected_portal_path("APPS/CALENDAR/EVENTS.TMP"));
        assert!(is_protected_portal_path("APPS/CALENDAR/EVENTS.BAK"));
        assert!(!is_protected_portal_path("APPS/CALENDAR/EVENTS.TXT"));
        assert!(!is_protected_portal_path("/VOICE/VOICE001.WAV"));
        assert!(!is_protected_portal_path("/BOOKS/NOTES001.TXT"));
    }

    #[test]
    fn query_parser_decodes_portal_paths() {
        assert_eq!(
            query_value("/api/list?code=123456&path=%2FBOOKS", "path").as_deref(),
            Some("/BOOKS")
        );
    }
}
