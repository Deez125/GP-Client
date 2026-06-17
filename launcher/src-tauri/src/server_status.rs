//! Minecraft Server List Ping (1.7+ JSON protocol): query a server's status
//! (online/offline, player count + sample, MOTD) and round-trip latency. Used
//! for the server status display, quick-join, and friends-on-servers.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_PORT: u16 = 25565;

/// Server status returned to the UI. `online: false` means the ping failed
/// (server down / unreachable); the other fields are then zeroed.
#[derive(Serialize)]
pub struct ServerStatus {
    pub online: bool,
    pub players_online: u32,
    pub players_max: u32,
    /// Player names from the status "sample" (often partial; may be empty).
    pub sample: Vec<String>,
    /// MOTD flattened to plain text.
    pub motd: String,
    /// Round-trip latency in ms (time to the status response).
    pub ping_ms: u64,
}

impl ServerStatus {
    fn offline() -> Self {
        ServerStatus {
            online: false,
            players_online: 0,
            players_max: 0,
            sample: Vec::new(),
            motd: String::new(),
            ping_ms: 0,
        }
    }
}

// --- status JSON (the subset we use) ---
#[derive(Deserialize)]
struct StatusJson {
    #[serde(default)]
    players: Option<Players>,
    #[serde(default)]
    description: Option<serde_json::Value>,
}
#[derive(Deserialize)]
struct Players {
    #[serde(default)]
    online: u32,
    #[serde(default)]
    max: u32,
    #[serde(default)]
    sample: Vec<SamplePlayer>,
}
#[derive(Deserialize)]
struct SamplePlayer {
    #[serde(default)]
    name: String,
}

fn write_varint(buf: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn write_string(buf: &mut Vec<u8>, s: &str) {
    write_varint(buf, s.len() as u32);
    buf.extend_from_slice(s.as_bytes());
}

/// Length-prefix a packet body (VarInt length + body).
fn framed(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len() + 5);
    write_varint(&mut out, body.len() as u32);
    out.extend_from_slice(body);
    out
}

async fn read_varint(stream: &mut TcpStream) -> Result<u32, String> {
    let mut result: u32 = 0;
    let mut shift = 0u32;
    loop {
        let byte = stream.read_u8().await.map_err(|e| e.to_string())?;
        result |= ((byte & 0x7F) as u32) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 32 {
            return Err("varint too long".to_string());
        }
    }
    Ok(result)
}

/// Split "host" or "host:port" into (host, port).
fn split_addr(address: &str) -> (String, u16) {
    match address.rsplit_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().unwrap_or(DEFAULT_PORT)),
        None => (address.to_string(), DEFAULT_PORT),
    }
}

async fn ping(address: &str) -> Result<ServerStatus, String> {
    let (host, port) = split_addr(address);
    let started = Instant::now();

    let mut stream = tokio::time::timeout(TIMEOUT, TcpStream::connect((host.as_str(), port)))
        .await
        .map_err(|_| "connection timed out".to_string())?
        .map_err(|e| format!("connect: {e}"))?;

    // Handshake: protocol version is arbitrary for a status ping; next state = 1.
    let mut hs = Vec::new();
    write_varint(&mut hs, 0x00);
    write_varint(&mut hs, 760);
    write_string(&mut hs, &host);
    hs.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut hs, 1);
    stream.write_all(&framed(&hs)).await.map_err(|e| e.to_string())?;

    // Status request (empty body, packet id 0x00).
    let mut req = Vec::new();
    write_varint(&mut req, 0x00);
    stream.write_all(&framed(&req)).await.map_err(|e| e.to_string())?;

    // Response: VarInt length, VarInt packet id, String(JSON).
    let data = tokio::time::timeout(TIMEOUT, async {
        let _len = read_varint(&mut stream).await?;
        let _pid = read_varint(&mut stream).await?;
        let json_len = read_varint(&mut stream).await? as usize;
        let mut data = vec![0u8; json_len];
        stream
            .read_exact(&mut data)
            .await
            .map_err(|e| e.to_string())?;
        Ok::<Vec<u8>, String>(data)
    })
    .await
    .map_err(|_| "response timed out".to_string())??;

    let ping_ms = started.elapsed().as_millis() as u64;

    let json: StatusJson =
        serde_json::from_slice(&data).map_err(|e| format!("parse status: {e}"))?;
    let players = json.players.unwrap_or(Players {
        online: 0,
        max: 0,
        sample: Vec::new(),
    });
    let sample = players
        .sample
        .into_iter()
        .map(|p| p.name)
        .filter(|n| !n.is_empty())
        .collect();
    let motd = json.description.map(motd_text).unwrap_or_default();

    Ok(ServerStatus {
        online: true,
        players_online: players.online,
        players_max: players.max,
        sample,
        motd,
        ping_ms,
    })
}

/// MOTD may be a plain string or a chat-component object; flatten to text.
fn motd_text(v: serde_json::Value) -> String {
    match &v {
        serde_json::Value::String(s) => s.clone(),
        _ => {
            let mut out = String::new();
            collect_text(&v, &mut out);
            out
        }
    }
}
fn collect_text(v: &serde_json::Value, out: &mut String) {
    if let Some(t) = v.get("text").and_then(|t| t.as_str()) {
        out.push_str(t);
    }
    if let Some(extra) = v.get("extra").and_then(|e| e.as_array()) {
        for child in extra {
            collect_text(child, out);
        }
    }
}

/// Query a server's status. Never errors — an unreachable server returns an
/// `offline` status so the UI can just render it.
#[tauri::command]
pub async fn server_status(address: String) -> Result<ServerStatus, String> {
    Ok(ping(&address).await.unwrap_or_else(|_| ServerStatus::offline()))
}
