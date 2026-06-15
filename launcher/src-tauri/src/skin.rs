//! Fetch a player's Minecraft skin via the public Mojang API and return just
//! the face (the 8x8 face region + the hat/overlay layer), scaled up with
//! nearest-neighbor, as a PNG data URL. Done in Rust to avoid browser CORS.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::STANDARD, Engine};
use image::{imageops, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

const SESSION_PROFILE: &str = "https://sessionserver.mojang.com/session/minecraft/profile/";
const FACE_SIZE: u32 = 128;

#[derive(Deserialize)]
struct Profile {
    properties: Vec<Property>,
}

#[derive(Deserialize)]
struct Property {
    name: String,
    value: String,
}

#[derive(Deserialize)]
struct TexturesProperty {
    textures: TextureMap,
}

#[derive(Deserialize)]
struct TextureMap {
    #[serde(rename = "SKIN")]
    skin: Option<SkinTexture>,
}

#[derive(Deserialize)]
struct SkinTexture {
    url: String,
}

/// Fetch a player's raw skin PNG bytes via the Mojang API.
async fn fetch_skin_png(uuid: &str) -> Result<Vec<u8>, String> {
    let uuid = uuid.replace('-', "");
    let client = reqwest::Client::builder()
        .user_agent("GPClient/0.1 (+launcher)")
        .build()
        .map_err(|e| e.to_string())?;

    // Profile -> base64 "textures" property -> skin URL.
    let profile: Profile = client
        .get(format!("{SESSION_PROFILE}{uuid}"))
        .send()
        .await
        .map_err(|e| format!("profile: {e}"))?
        .json()
        .await
        .map_err(|e| format!("profile json: {e}"))?;

    let prop = profile
        .properties
        .iter()
        .find(|p| p.name == "textures")
        .ok_or("no textures property")?;
    let decoded = STANDARD
        .decode(prop.value.as_bytes())
        .map_err(|e| format!("textures b64: {e}"))?;
    let textures: TexturesProperty =
        serde_json::from_slice(&decoded).map_err(|e| format!("textures json: {e}"))?;
    let skin_url = textures
        .textures
        .skin
        .ok_or("account has no custom skin")?
        .url;

    let bytes = client
        .get(&skin_url)
        .send()
        .await
        .map_err(|e| format!("skin download: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("skin bytes: {e}"))?;
    Ok(bytes.to_vec())
}

fn data_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", STANDARD.encode(png))
}

/// Return the full skin PNG as a `data:image/png;base64,...` URL (for the 3D
/// viewer — a data URL avoids canvas/WebGL CORS issues).
#[tauri::command]
pub async fn get_skin(uuid: String) -> Result<String, String> {
    let png = fetch_skin_png(&uuid).await?;
    Ok(data_url(&png))
}

/// Return just the player's face (face + hat overlay, scaled up) as a data URL.
#[tauri::command]
pub async fn get_skin_face(uuid: String) -> Result<String, String> {
    let bytes = fetch_skin_png(&uuid).await?;
    let skin = image::load_from_memory(&bytes)
        .map_err(|e| format!("decode skin: {e}"))?
        .to_rgba8();
    let face = compose_face(&skin);
    let scaled = imageops::resize(&face, FACE_SIZE, FACE_SIZE, imageops::FilterType::Nearest);

    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(scaled)
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| format!("encode png: {e}"))?;
    Ok(data_url(buf.get_ref()))
}

/// Compose the 8x8 face (at 8,8) with the hat/overlay layer (at 40,8) on top.
fn compose_face(skin: &RgbaImage) -> RgbaImage {
    let mut out = RgbaImage::new(8, 8);
    for y in 0..8 {
        for x in 0..8 {
            let base = *skin.get_pixel(8 + x, 8 + y);
            let hat = *skin.get_pixel(40 + x, 8 + y);
            out.put_pixel(x, y, if hat[3] > 0 { blend(base, hat) } else { base });
        }
    }
    out
}

/// Alpha-blend `over` onto `base`, returning an opaque pixel.
fn blend(base: Rgba<u8>, over: Rgba<u8>) -> Rgba<u8> {
    let a = over[3] as f32 / 255.0;
    let inv = 1.0 - a;
    Rgba([
        (over[0] as f32 * a + base[0] as f32 * inv) as u8,
        (over[1] as f32 * a + base[1] as f32 * inv) as u8,
        (over[2] as f32 * a + base[2] as f32 * inv) as u8,
        255,
    ])
}

// ============================================================================
// Skin library — persisted under `<.minecraft>/GP Client/skins/` (the shared
// folder, NOT installations). Seeded with Steve, Alex, and a snapshot of the
// player's current skin. Names are editable; entries are sorted newest-first.
// ============================================================================

const STEVE_PNG: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/seed/steve.png"));
const ALEX_PNG: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/seed/alex.png"));

#[derive(Serialize, Deserialize, Clone)]
struct StoredSkin {
    id: String,
    name: String,
    file: String,
    #[serde(default = "default_model")]
    model: String,
    added: u64,
}

fn default_model() -> String {
    "auto-detect".to_string()
}

/// A library skin sent to the UI (PNG inlined as a data URL for rendering).
#[derive(Serialize)]
pub struct SkinLibEntry {
    id: String,
    name: String,
    model: String,
    added: u64,
    data_url: String,
}

fn skins_dir() -> PathBuf {
    crate::installations::shared_root().join("skins")
}

fn index_path() -> PathBuf {
    skins_dir().join("skins.json")
}

fn read_index() -> Vec<StoredSkin> {
    std::fs::read_to_string(index_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn write_index(list: &[StoredSkin]) -> Result<(), String> {
    std::fs::create_dir_all(skins_dir()).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    std::fs::write(index_path(), text).map_err(|e| e.to_string())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// List the saved skins (seeding Steve/Alex/Current on first run), newest first.
#[tauri::command]
pub async fn list_skins(uuid: String) -> Result<Vec<SkinLibEntry>, String> {
    std::fs::create_dir_all(skins_dir()).map_err(|e| e.to_string())?;
    let mut index = read_index();
    let mut changed = false;

    if !index.iter().any(|s| s.id == "steve") {
        std::fs::write(skins_dir().join("steve.png"), STEVE_PNG).map_err(|e| e.to_string())?;
        index.push(StoredSkin {
            id: "steve".into(),
            name: "Steve".into(),
            file: "steve.png".into(),
            model: "default".into(),
            added: 1,
        });
        changed = true;
    }
    if !index.iter().any(|s| s.id == "alex") {
        std::fs::write(skins_dir().join("alex.png"), ALEX_PNG).map_err(|e| e.to_string())?;
        index.push(StoredSkin {
            id: "alex".into(),
            name: "Alex".into(),
            file: "alex.png".into(),
            model: "slim".into(),
            added: 2,
        });
        changed = true;
    }
    if !index.iter().any(|s| s.id == "current") {
        // Snapshot the player's current Mojang skin (best-effort).
        if let Ok(png) = fetch_skin_png(&uuid).await {
            std::fs::write(skins_dir().join("current.png"), &png).map_err(|e| e.to_string())?;
            index.push(StoredSkin {
                id: "current".into(),
                name: "Current".into(),
                file: "current.png".into(),
                model: "auto-detect".into(),
                added: now_secs(),
            });
            changed = true;
        }
    }
    if changed {
        write_index(&index)?;
    }

    // Newest first.
    index.sort_by(|a, b| b.added.cmp(&a.added));

    let mut out = Vec::new();
    for s in &index {
        let bytes = std::fs::read(skins_dir().join(&s.file)).map_err(|e| e.to_string())?;
        out.push(SkinLibEntry {
            id: s.id.clone(),
            name: s.name.clone(),
            model: s.model.clone(),
            added: s.added,
            data_url: data_url(&bytes),
        });
    }
    Ok(out)
}

/// Rename a saved skin (empty name is ignored).
#[tauri::command]
pub fn rename_skin(id: String, name: String) -> Result<(), String> {
    let mut index = read_index();
    let entry = index
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or("skin not found")?;
    let trimmed = name.trim();
    if !trimmed.is_empty() {
        entry.name = trimmed.to_string();
    }
    write_index(&index)
}
