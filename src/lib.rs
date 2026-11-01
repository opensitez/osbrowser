//! The system browser, behind webcore's API.
//!
//! The same modules, types and signatures as webcore — `types::Document`,
//! `dom`, `window`, `browser`, `canvas`, `ui_events`, `scheduling`,
//! `platform::dialogs` — so a program written against one compiles against the
//! other. The page runs in the user's own browser (whatever the platform opens
//! for a URL); this process drives it over a loopback HTTP channel, and DOM
//! events come back on the same channel. No browser executable, UI toolkit or
//! rendering engine is linked.

pub mod browser;
pub mod canvas;
pub mod dom;
pub mod embedded_window;
pub mod html;
pub mod layout;
mod page;
pub mod platform;
pub mod renderer;
pub mod scheduling;
pub mod types;
pub mod ui_events;
pub mod window;

pub use types::{Document, FormEvent, FormEventCallback, FormEventKind, Rect};

/// Parse `html` into a new document, laid out against `viewport_width`.
pub fn load_html(html: &str, viewport_width: f32) -> Document {
    Document::load(html, viewport_width, 700.0)
}

/// The browser fetches a page's images itself, and starts again by itself when
/// a source changes, so there is nothing to restart from here.
pub fn restart_async_image_fetches(_doc: &mut types::Document) {}

use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::Command;
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BrowserEvent {
    pub document: u64,
    pub node: u64,
    #[serde(default)]
    pub path: Vec<u64>,
    pub kind: String,
    #[serde(default)]
    pub fields: Value,
    /// The `add_event_listener` registration this delivery is for; `None` for
    /// the page-wide report every event also produces.
    #[serde(default)]
    pub listener: Option<u32>,
}

#[derive(Serialize)]
struct WireCommand {
    id: u64,
    document: u64,
    operation: String,
    args: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    assigned_node: Option<u64>,
    #[serde(skip)]
    pixels: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    binary_range: Option<[usize; 2]>,
}

struct HttpResponse {
    status: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
}

impl HttpResponse {
    fn ok(body: impl Into<Vec<u8>>, content_type: &'static str) -> Self {
        Self {
            status: "200 OK",
            content_type,
            body: body.into(),
        }
    }

    fn no_content() -> Self {
        Self {
            status: "204 No Content",
            content_type: "application/json",
            body: Vec::new(),
        }
    }

    fn not_found() -> Self {
        Self {
            status: "404 Not Found",
            content_type: "text/plain",
            body: b"not found".to_vec(),
        }
    }
}

fn command_response(mut commands: Vec<WireCommand>) -> HttpResponse {
    if !commands.iter().any(|command| command.pixels.is_some()) {
        return HttpResponse::ok(
            serde_json::to_vec(&commands).unwrap_or_default(),
            "application/json",
        );
    }
    let mut offset = 0;
    for command in &mut commands {
        if let Some(pixels) = &command.pixels {
            command.binary_range = Some([offset, pixels.len()]);
            offset += pixels.len();
        }
    }
    let header = serde_json::to_vec(&commands).expect("serializable commands");
    let header_len = header.len() as u32;
    let mut body = Vec::with_capacity(4 + header.len() + offset);
    body.extend_from_slice(&header_len.to_le_bytes());
    body.extend_from_slice(&header);
    for command in commands {
        if let Some(pixels) = command.pixels {
            body.extend_from_slice(&pixels);
        }
    }
    HttpResponse::ok(body, "application/octet-stream")
}

#[derive(Deserialize)]
struct WireReply {
    id: u64,
    result: Value,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum EventPacket {
    Batch(Vec<BrowserEvent>),
    Single(BrowserEvent),
}

struct State {
    next: u64,
    commands: VecDeque<WireCommand>,
    replies: HashMap<u64, mpsc::Sender<Result<Value, String>>>,
    events: VecDeque<BrowserEvent>,
    connected: bool,
    last_seen: Option<Instant>,
    batch_depth: usize,
    flush_requested: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            next: 1,
            commands: VecDeque::new(),
            replies: HashMap::new(),
            events: VecDeque::new(),
            connected: false,
            last_seen: None,
            batch_depth: 0,
            flush_requested: false,
        }
    }
}

impl State {
    fn take_ready_commands(&mut self) -> Vec<WireCommand> {
        self.last_seen = Some(Instant::now());
        if self.batch_depth > 0 && !self.flush_requested {
            return Vec::new();
        }
        self.flush_requested = false;
        self.commands.drain(..).collect()
    }
}

/// One browser tab and its bidirectional command channel.
pub struct BrowserSession {
    address: SocketAddr,
    token: String,
    state: Arc<Mutex<State>>,
    event_ready: Arc<Condvar>,
    wake: Arc<Condvar>,
    /// The headless browser hosting the page, when [`Self::open_headless`]
    /// started one. Held so it lives exactly as long as this process.
    headless: Mutex<Option<std::process::Child>>,
}

/// A browser that can host a page with no window: its executable, and the
/// arguments that run it headless on `url` with a fresh profile in `profile`.
struct HeadlessBrowser {
    program: std::path::PathBuf,
    firefox: bool,
}

impl HeadlessBrowser {
    /// Chrome or Chromium first — the bridge is exercised against Chrome —
    /// then Firefox.
    fn find() -> Option<Self> {
        let chrome: &[&str] = if cfg!(target_os = "macos") {
            &["/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
              "/Applications/Chromium.app/Contents/MacOS/Chromium"]
        } else if cfg!(target_os = "windows") {
            &[r"C:\Program Files\Google\Chrome\Application\chrome.exe",
              r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe"]
        } else {
            &["google-chrome", "google-chrome-stable", "chromium", "chromium-browser"]
        };
        let firefox: &[&str] = if cfg!(target_os = "macos") {
            &["/Applications/Firefox.app/Contents/MacOS/firefox"]
        } else if cfg!(target_os = "windows") {
            &[r"C:\Program Files\Mozilla Firefox\firefox.exe"]
        } else {
            &["firefox"]
        };
        let locate = |name: &str| -> Option<std::path::PathBuf> {
            let path = std::path::Path::new(name);
            if path.is_absolute() {
                return path.is_file().then(|| path.to_path_buf());
            }
            std::env::split_paths(&std::env::var_os("PATH")?)
                .map(|dir| dir.join(name))
                .find(|candidate| candidate.is_file())
        };
        chrome.iter().find_map(|name| locate(name)).map(|program| Self { program, firefox: false })
            .or_else(|| firefox.iter().find_map(|name| locate(name)).map(|program| Self { program, firefox: true }))
    }

    fn args(&self, url: &str, profile: &std::path::Path) -> Vec<String> {
        let profile = profile.display().to_string();
        if self.firefox {
            vec!["--headless".into(), "-no-remote".into(), "-profile".into(), profile, url.into()]
        } else {
            vec!["--headless=new".into(), format!("--user-data-dir={profile}"),
                 "--no-first-run".into(), "--no-default-browser-check".into(), url.into()]
        }
    }
}

fn random_token() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    format!("{pid:x}{now:x}")
}

impl BrowserSession {
    /// Start a loopback server. The browser is opened separately with `open`.
    pub fn start() -> Result<Self, String> {
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let state = Arc::new(Mutex::new(State::default()));
        let server_state = Arc::clone(&state);
        let event_ready = Arc::new(Condvar::new());
        let server_event_ready = Arc::clone(&event_ready);
        let token = random_token();
        let server_token = token.clone();
        let wake = Arc::new(Condvar::new());
        let server_wake = Arc::clone(&wake);

        std::thread::Builder::new()
            .name("osbrowser-http".into())
            .spawn(move || {
                let listener = match TcpListener::bind("127.0.0.1:0") {
                    Ok(l) => l,
                    Err(e) => {
                        let _ = ready_tx.send(Err(format!("cannot bind browser bridge: {e}")));
                        return;
                    }
                };
                let Ok(addr) = listener.local_addr() else {
                    let _ = ready_tx.send(Err("cannot get local address".into()));
                    return;
                };
                let _ = ready_tx.send(Ok(addr));

                for stream in listener.incoming() {
                    let Ok(stream) = stream else { continue };
                    let state = Arc::clone(&server_state);
                    let event_ready = Arc::clone(&server_event_ready);
                    let token = server_token.clone();
                    let wake = Arc::clone(&server_wake);
                    std::thread::spawn(move || {
                        handle_connection(stream, &state, &token, &wake, &event_ready);
                    });
                }
            })
            .map_err(|e| e.to_string())?;

        Ok(Self {
            address: ready_rx.recv().map_err(|e| e.to_string())??,
            token,
            state,
            event_ready,
            wake,
            headless: Mutex::new(None),
        })
    }

    pub fn url(&self) -> String {
        format!("http://{}/{}/", self.address, self.token)
    }

    /// Open an external browser window. Prefer Chrome on macOS, where the
    /// bridge is exercised against Chrome; fall back to the system browser.
    pub fn open(&self) -> Result<(), String> {
        let url = self.url();
        #[cfg(target_os = "macos")]
        let status = if std::path::Path::new("/Applications/Google Chrome.app").exists() {
            Command::new("open")
                .args(["-a", "Google Chrome", &url])
                .status()
        } else {
            Command::new("open").arg(&url).status()
        };
        #[cfg(target_os = "windows")]
        let status = Command::new("cmd").args(["/C", "start", "", &url]).status();
        #[cfg(all(unix, not(target_os = "macos")))]
        let status = Command::new("xdg-open").arg(&url).status();
        #[cfg(not(any(unix, target_os = "windows")))]
        let status: std::io::Result<std::process::ExitStatus> =
            Err(std::io::Error::other("unsupported platform"));
        match status {
            Ok(code) if code.success() => Ok(()),
            Ok(code) => Err(format!("browser launcher exited with {code}")),
            Err(error) => Err(error.to_string()),
        }
    }

    /// Launch a headless Chrome or Firefox in the background, pointed at this
    /// session's URL. The process is killed when this session is dropped.
    pub fn open_headless(&self) -> Result<(), String> {
        let mut slot = self.headless.lock().map_err(|e| e.to_string())?;
        if slot.is_some() {
            return Ok(());
        }
        let browser = HeadlessBrowser::find().ok_or("no headless Chrome or Firefox found")?;
        let profile = std::env::temp_dir().join(format!("osbrowser-{}", random_token()));
        let args = browser.args(&self.url(), &profile);
        #[cfg(unix)]
        let child = Command::new(&browser.program)
            .args(&args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        #[cfg(not(unix))]
        let child = Command::new(&browser.program)
            .args(&args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        *slot = Some(child.map_err(|e| format!("cannot start {}: {e}", browser.program.display()))?);
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.state.lock().is_ok_and(|state| {
            state.connected
                && state
                    .last_seen
                    .is_some_and(|at| at.elapsed() < Duration::from_secs(10))
        })
    }

    pub fn begin_batch(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.batch_depth += 1;
        }
    }

    pub fn end_batch(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.batch_depth = state.batch_depth.saturating_sub(1);
            if state.batch_depth == 0 && !state.commands.is_empty() {
                drop(state);
                self.wake.notify_all();
            }
        }
    }

    pub fn flush(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.flush_requested = true;
            drop(state);
            self.wake.notify_all();
        }
    }

    /// Send a DOM operation and wait for the browser's reply.
    pub fn call(&self, document: u64, operation: &str, args: Value) -> Result<Value, String> {
        self.call_payload(document, operation, args, None)
    }

    /// Upload raw RGBA bytes using standard Canvas ImageData metadata.
    pub fn call_with_pixels(
        &self,
        document: u64,
        args: Value,
        pixels: Vec<u8>,
    ) -> Result<Value, String> {
        self.call_payload(document, "CanvasApply", args, Some(pixels))
    }

    fn call_payload(
        &self,
        document: u64,
        operation: &str,
        args: Value,
        pixels: Option<Vec<u8>>,
    ) -> Result<Value, String> {
        let (tx, rx) = mpsc::channel();
        let id = {
            let mut state = self.state.lock().map_err(|e| e.to_string())?;
            let id = state.next;
            state.next += 1;
            state.replies.insert(id, tx);
            state.commands.push_back(WireCommand {
                id,
                document,
                operation: operation.to_string(),
                args,
                assigned_node: None,
                pixels,
                binary_range: None,
            });
            state.flush_requested = true;
            id
        };
        self.wake.notify_all();
        let result = rx.recv_timeout(Duration::from_secs(10));
        if result.is_err() {
            if let Ok(mut state) = self.state.lock() {
                state.replies.remove(&id);
                state.commands.retain(|command| command.id != id);
            }
        }
        result.map_err(|_| format!("browser did not answer {operation}"))?
    }

    /// Queue an ordered browser mutation without waiting for a round trip.
    /// The next `call` observes all preceding queued operations.
    pub fn enqueue(
        &self,
        document: u64,
        operation: &str,
        args: Value,
        assigned_node: Option<u64>,
    ) -> Result<(), String> {
        self.enqueue_payload(document, operation, args, assigned_node, None)
    }

    /// Queue a raw RGBA upload in the same ordered stream as DOM mutations.
    pub fn enqueue_with_pixels(
        &self,
        document: u64,
        args: Value,
        pixels: Vec<u8>,
    ) -> Result<(), String> {
        self.enqueue_payload(document, "CanvasApply", args, None, Some(pixels))
    }

    pub(crate) fn enqueue_payload(
        &self,
        document: u64,
        operation: &str,
        args: Value,
        assigned_node: Option<u64>,
        pixels: Option<Vec<u8>>,
    ) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        state.commands.push_back(WireCommand {
            id: 0,
            document,
            operation: operation.to_string(),
            args,
            assigned_node,
            pixels,
            binary_range: None,
        });
        let ready = state.batch_depth == 0;
        drop(state);
        if ready {
            self.wake.notify_all();
        }
        Ok(())
    }

    pub fn drain_events(&self) -> Vec<BrowserEvent> {
        self.state
            .lock()
            .map(|mut state| state.events.drain(..).collect())
            .unwrap_or_default()
    }

    pub fn wait_for_event(&self, timeout: Duration) {
        if let Ok(state) = self.state.lock() {
            if state.events.is_empty() {
                let _ = self
                    .event_ready
                    .wait_timeout_while(state, timeout, |state| state.events.is_empty());
            }
        }
    }
}

fn handle_connection(
    mut stream: TcpStream,
    state: &Arc<Mutex<State>>,
    token: &str,
    wake: &Arc<Condvar>,
    event_ready: &Arc<Condvar>,
) {
    let mut buf = [0u8; 4096];
    let n = match stream.read(&mut buf) {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    let req_str = String::from_utf8_lossy(&buf[..n]);
    let mut lines = req_str.lines();
    let Some(first_line) = lines.next() else { return };
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let full_path = parts.next().unwrap_or("");

    let mut content_length = 0;
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some(val) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            content_length = val.trim().parse::<usize>().unwrap_or(0);
        }
    }

    let header_end = req_str
        .find("\r\n\r\n")
        .map(|p| p + 4)
        .or_else(|| req_str.find("\n\n").map(|p| p + 2))
        .unwrap_or(n);

    let mut body = buf[header_end..n].to_vec();
    while body.len() < content_length {
        let mut extra = vec![0u8; (content_length - body.len()).min(8192)];
        match stream.read(&mut extra) {
            Ok(read_bytes) if read_bytes > 0 => {
                body.extend_from_slice(&extra[..read_bytes]);
            }
            _ => break,
        }
    }

    let prefix = format!("/{token}");
    let path = full_path.strip_prefix(&prefix).unwrap_or("");
    let path = path.split('?').next().unwrap_or(path);

    let resp = match (method, path) {
        ("GET", "" | "/") => HttpResponse::ok(
            include_str!("bridge.html").as_bytes(),
            "text/html; charset=utf-8",
        ),
        ("GET", "/bridge.js") => HttpResponse::ok(
            include_str!("bridge.js").as_bytes(),
            "text/javascript; charset=utf-8",
        ),
        ("GET", "/canvas.js") => HttpResponse::ok(
            include_str!("canvas.js").as_bytes(),
            "text/javascript; charset=utf-8",
        ),
        ("GET", "/next") => {
            let mut s = state.lock().unwrap();
            s.connected = true;
            let mut commands = s.take_ready_commands();
            if commands.is_empty() {
                let (guard, _) = wake.wait_timeout(s, Duration::from_secs(1)).unwrap();
                s = guard;
                commands = s.take_ready_commands();
            }
            drop(s);
            if commands.is_empty() {
                HttpResponse::no_content()
            } else {
                command_response(commands)
            }
        }
        ("POST", "/reply") => {
            if let Ok(replies) = serde_json::from_slice::<Vec<WireReply>>(&body) {
                if let Ok(mut s) = state.lock() {
                    for reply in replies {
                        if let Some(tx) = s.replies.remove(&reply.id) {
                            let _ = tx.send(match reply.error {
                                Some(err) => Err(err),
                                None => Ok(reply.result),
                            });
                        }
                    }
                }
            }
            HttpResponse::no_content()
        }
        ("POST", "/event") => {
            if let Ok(packet) = serde_json::from_slice::<EventPacket>(&body) {
                if let Ok(mut s) = state.lock() {
                    match packet {
                        EventPacket::Batch(events) => s.events.extend(events),
                        EventPacket::Single(event) => s.events.push_back(event),
                    }
                    event_ready.notify_one();
                }
            }
            HttpResponse::no_content()
        }
        _ => HttpResponse::not_found(),
    };

    let head = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        resp.status, resp.content_type, resp.body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&resp.body);
    let _ = stream.flush();
}

#[cfg(test)]
mod tests {
    use super::{BrowserEvent, BrowserSession, State, WireCommand};
    use std::sync::{Arc, Condvar, Mutex};
    use std::time::{Duration, Instant};

    fn command(id: u64, operation: &str) -> WireCommand {
        WireCommand {
            id,
            document: 1,
            operation: operation.into(),
            args: serde_json::Value::Null,
            assigned_node: None,
            pixels: None,
            binary_range: None,
        }
    }

    #[test]
    fn binary_response_preserves_order_and_raw_pixel_ranges() {
        let mut first = command(1, "CanvasApply");
        first.pixels = Some(vec![1, 2, 3, 255]);
        let second = command(2, "Title");
        let mut third = command(3, "CanvasApply");
        third.pixels = Some(vec![4, 5, 6, 255]);
        let response = super::command_response(vec![first, second, third]);
        assert_eq!(response.content_type, "application/octet-stream");
        let body = response.body;
        let length = u32::from_le_bytes(body[..4].try_into().unwrap()) as usize;
        let commands: serde_json::Value = serde_json::from_slice(&body[4..4 + length]).unwrap();
        assert_eq!(commands[0]["binary_range"], serde_json::json!([0, 4]));
        assert!(commands[1].get("binary_range").is_none());
        assert_eq!(commands[2]["binary_range"], serde_json::json!([4, 4]));
        assert_eq!(&body[4 + length..], &[1, 2, 3, 255, 4, 5, 6, 255]);
    }

    #[test]
    fn mutations_wait_for_batch_end() {
        let mut state = State::default();
        state.batch_depth = 1;
        state.commands.push_back(command(0, "SetStyleProperty"));
        assert!(state.take_ready_commands().is_empty());
        state.batch_depth = 0;
        assert_eq!(state.take_ready_commands()[0].operation, "SetStyleProperty");
    }

    #[test]
    fn grid_creation_drains_as_one_ordered_batch() {
        let mut state = State::default();
        state.batch_depth = 1;
        for _ in 0..42 {
            state.commands.push_back(command(0, "CreateElement"));
            state.commands.push_back(command(0, "SetStyleProperty"));
            state.commands.push_back(command(0, "SetStyleProperty"));
            state.commands.push_back(command(0, "AppendChild"));
            assert!(state.take_ready_commands().is_empty());
        }
        state.batch_depth = 0;
        let commands = state.take_ready_commands();
        assert_eq!(commands.len(), 42 * 4);
        for cell in commands.chunks_exact(4) {
            assert_eq!(cell[0].operation, "CreateElement");
            assert_eq!(cell[3].operation, "AppendChild");
        }
        assert!(state.take_ready_commands().is_empty());
    }

    #[test]
    fn read_flushes_earlier_mutations_in_order() {
        let mut state = State::default();
        state.batch_depth = 1;
        state.commands.push_back(command(0, "CreateElement"));
        state.commands.push_back(command(0, "AppendChild"));
        state.commands.push_back(command(7, "QuerySelector"));
        state.flush_requested = true;
        let operations: Vec<_> = state
            .take_ready_commands()
            .into_iter()
            .map(|command| command.operation)
            .collect();
        assert_eq!(
            operations,
            ["CreateElement", "AppendChild", "QuerySelector"]
        );
        state.commands.push_back(command(0, "SetStyleProperty"));
        assert!(state.take_ready_commands().is_empty());
    }

    #[test]
    fn browser_event_wakes_idle_vm() {
        let session = BrowserSession {
            address: "127.0.0.1:0".parse().unwrap(),
            token: String::new(),
            state: Arc::new(Mutex::new(State::default())),
            event_ready: Arc::new(Condvar::new()),
            wake: Arc::new(Condvar::new()),
            headless: Mutex::new(None),
        };
        let state = Arc::clone(&session.state);
        let ready = Arc::clone(&session.event_ready);
        let producer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(10));
            state.lock().unwrap().events.push_back(BrowserEvent {
                document: 1,
                node: 2,
                path: vec![2],
                kind: "click".into(),
                fields: serde_json::Value::Null,
                listener: None,
            });
            ready.notify_one();
        });
        let start = Instant::now();
        session.wait_for_event(Duration::from_secs(1));
        assert!(start.elapsed() < Duration::from_millis(500));
        assert_eq!(session.drain_events()[0].kind, "click");
        producer.join().unwrap();
    }
}
