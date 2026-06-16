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
    #[serde(rename = "CAPE", default)]
    cape: Option<SkinTexture>,
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

/// The player's skin and (optional) active cape, both as data URLs, for the 3D
/// viewer so it can render the cape too.
#[derive(Serialize)]
pub struct PlayerTextures {
    skin: String,
    cape: Option<String>,
}

#[tauri::command]
pub async fn get_player_textures(uuid: String) -> Result<PlayerTextures, String> {
    let uuid = uuid.replace('-', "");
    let client = reqwest::Client::builder()
        .user_agent("GPClient/0.1 (+launcher)")
        .build()
        .map_err(|e| e.to_string())?;

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
    let skin_bytes = client
        .get(&skin_url)
        .send()
        .await
        .map_err(|e| format!("skin download: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("skin bytes: {e}"))?;

    // Cape is best-effort; absence just means no cape on the model.
    let cape = match textures.textures.cape {
        Some(c) => match client.get(&c.url).send().await {
            Ok(r) => r.bytes().await.ok().map(|b| data_url(&b)),
            Err(_) => None,
        },
        None => None,
    };

    Ok(PlayerTextures {
        skin: data_url(&skin_bytes),
        cape,
    })
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
    /// Preferred cape for this skin: a cape id, "none", or null (leave unchanged).
    #[serde(default)]
    cape: Option<String>,
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
    cape: Option<String>,
}

/// Folder-safe key for an account UUID (no dashes, lowercase).
fn uuid_key(uuid: &str) -> String {
    uuid.replace('-', "").to_lowercase()
}

/// The skin library lives per-account, so switching accounts shows a separate
/// library: `<.minecraft>/GP Client/skins/<uuid>/`.
fn skins_dir(uuid: &str) -> PathBuf {
    crate::installations::shared_root()
        .join("skins")
        .join(uuid_key(uuid))
}

fn index_path(uuid: &str) -> PathBuf {
    skins_dir(uuid).join("skins.json")
}

fn read_index(uuid: &str) -> Vec<StoredSkin> {
    std::fs::read_to_string(index_path(uuid))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn write_index(uuid: &str, list: &[StoredSkin]) -> Result<(), String> {
    std::fs::create_dir_all(skins_dir(uuid)).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    std::fs::write(index_path(uuid), text).map_err(|e| e.to_string())
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
    std::fs::create_dir_all(skins_dir(&uuid)).map_err(|e| e.to_string())?;
    let mut index = read_index(&uuid);
    let mut changed = false;

    if !index.iter().any(|s| s.id == "steve") {
        std::fs::write(skins_dir(&uuid).join("steve.png"), STEVE_PNG).map_err(|e| e.to_string())?;
        index.push(StoredSkin {
            id: "steve".into(),
            name: "Steve".into(),
            file: "steve.png".into(),
            model: "default".into(),
            added: 1,
            cape: None,
        });
        changed = true;
    }
    if !index.iter().any(|s| s.id == "alex") {
        std::fs::write(skins_dir(&uuid).join("alex.png"), ALEX_PNG).map_err(|e| e.to_string())?;
        index.push(StoredSkin {
            id: "alex".into(),
            name: "Alex".into(),
            file: "alex.png".into(),
            model: "slim".into(),
            added: 2,
            cape: None,
        });
        changed = true;
    }
    // Keep the "Current" entry in sync with the live account skin on every open
    // (best-effort — a network hiccup just leaves the last known snapshot).
    if let Ok(png) = fetch_skin_png(&uuid).await {
        let _ = std::fs::write(skins_dir(&uuid).join("current.png"), &png);
        if !index.iter().any(|s| s.id == "current") {
            index.push(StoredSkin {
                id: "current".into(),
                name: "Current".into(),
                file: "current.png".into(),
                model: "auto-detect".into(),
                added: now_secs(),
                cape: None,
            });
            changed = true;
        }
    }
    if changed {
        write_index(&uuid, &index)?;
    }

    // Newest first.
    index.sort_by(|a, b| b.added.cmp(&a.added));

    let mut out = Vec::new();
    for s in &index {
        let bytes = std::fs::read(skins_dir(&uuid).join(&s.file)).map_err(|e| e.to_string())?;
        out.push(SkinLibEntry {
            id: s.id.clone(),
            name: s.name.clone(),
            model: s.model.clone(),
            added: s.added,
            data_url: data_url(&bytes),
            cape: s.cape.clone(),
        });
    }
    Ok(out)
}

/// Rename a saved skin (empty name is ignored).
#[tauri::command]
pub fn rename_skin(uuid: String, id: String, name: String) -> Result<(), String> {
    let mut index = read_index(&uuid);
    let entry = index
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or("skin not found")?;
    let trimmed = name.trim();
    if !trimmed.is_empty() {
        entry.name = trimmed.to_string();
    }
    write_index(&uuid, &index)
}

const PROFILE_SKINS_API: &str = "https://api.minecraftservices.com/minecraft/profile/skins";

/// Apply a library skin to the signed-in account by uploading it to Mojang.
/// `variant` is "classic" or "slim"; anything else falls back to classic.
#[tauri::command]
pub async fn apply_skin(uuid: String, id: String, variant: String) -> Result<(), String> {
    // Read the skin PNG from the library.
    let entry = read_index(&uuid)
        .into_iter()
        .find(|s| s.id == id)
        .ok_or("skin not found")?;
    let png = std::fs::read(skins_dir(&uuid).join(&entry.file))
        .map_err(|e| format!("read skin file: {e}"))?;

    // Fresh Minecraft token from the signed-in account.
    let profile = crate::auth::login_silent()
        .await
        .map_err(|e| format!("auth: {e}"))?
        .ok_or("You must be signed in to change your skin.")?;

    let variant = if variant.eq_ignore_ascii_case("slim") {
        "slim"
    } else {
        "classic"
    };

    let client = reqwest::Client::builder()
        .user_agent("GPClient/0.1 (+launcher)")
        .build()
        .map_err(|e| e.to_string())?;

    let part = reqwest::multipart::Part::bytes(png)
        .file_name("skin.png")
        .mime_str("image/png")
        .map_err(|e| e.to_string())?;
    let form = reqwest::multipart::Form::new()
        .text("variant", variant)
        .part("file", part);

    let resp = client
        .post(PROFILE_SKINS_API)
        .bearer_auth(&profile.access_token)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("upload skin: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Mojang rejected the skin (HTTP {status}). {body}"));
    }
    Ok(())
}

/// Import a PNG file from disk into the skin library. Validates it's a Minecraft
/// skin (64x64, or legacy 64x32) before adding. Returns the new skin's id.
#[tauri::command]
pub fn import_skin(
    uuid: String,
    path: String,
    name: String,
    model: String,
) -> Result<String, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("read file: {e}"))?;
    let img = image::load_from_memory(&bytes).map_err(|e| format!("not a valid image: {e}"))?;
    let (w, h) = (img.width(), img.height());
    if w != 64 || (h != 64 && h != 32) {
        return Err(format!(
            "not a Minecraft skin: expected 64x64 (or 64x32), got {w}x{h}"
        ));
    }

    std::fs::create_dir_all(skins_dir(&uuid)).map_err(|e| e.to_string())?;

    // Unique id + filename (timestamp + short random suffix to avoid clashes).
    let suffix: u32 = rand::random();
    let id = format!("skin-{}-{:08x}", now_secs(), suffix);
    let file = format!("{id}.png");
    std::fs::write(skins_dir(&uuid).join(&file), &bytes).map_err(|e| format!("save skin: {e}"))?;

    let display = {
        let t = name.trim();
        if t.is_empty() {
            "Imported skin".to_string()
        } else {
            t.to_string()
        }
    };
    let model = if model.eq_ignore_ascii_case("slim") {
        "slim"
    } else {
        "default"
    };

    let mut index = read_index(&uuid);
    index.push(StoredSkin {
        id: id.clone(),
        name: display,
        file,
        model: model.to_string(),
        added: now_secs(),
        cape: None,
    });
    write_index(&uuid, &index)?;
    Ok(id)
}

/// Update a library skin's metadata (name, model, cape preference).
#[tauri::command]
pub fn update_skin(
    uuid: String,
    id: String,
    name: String,
    model: String,
    cape: Option<String>,
) -> Result<(), String> {
    let mut index = read_index(&uuid);
    let entry = index.iter_mut().find(|s| s.id == id).ok_or("skin not found")?;
    let trimmed = name.trim();
    if !trimmed.is_empty() {
        entry.name = trimmed.to_string();
    }
    entry.model = if model.eq_ignore_ascii_case("slim") {
        "slim".to_string()
    } else {
        "default".to_string()
    };
    entry.cape = cape;
    write_index(&uuid, &index)
}

/// Replace a library skin's PNG with the file at `path` (validated as a skin).
#[tauri::command]
pub fn replace_skin_file(uuid: String, id: String, path: String) -> Result<(), String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("read file: {e}"))?;
    let img = image::load_from_memory(&bytes).map_err(|e| format!("not a valid image: {e}"))?;
    let (w, h) = (img.width(), img.height());
    if w != 64 || (h != 64 && h != 32) {
        return Err(format!(
            "not a Minecraft skin: expected 64x64 (or 64x32), got {w}x{h}"
        ));
    }
    let index = read_index(&uuid);
    let entry = index.iter().find(|s| s.id == id).ok_or("skin not found")?;
    std::fs::write(skins_dir(&uuid).join(&entry.file), &bytes)
        .map_err(|e| format!("save skin: {e}"))?;
    Ok(())
}

/// Delete a library skin (its index entry and PNG file).
#[tauri::command]
pub fn delete_skin(uuid: String, id: String) -> Result<(), String> {
    let mut index = read_index(&uuid);
    if let Some(pos) = index.iter().position(|s| s.id == id) {
        let removed = index.remove(pos);
        let _ = std::fs::remove_file(skins_dir(&uuid).join(&removed.file));
        write_index(&uuid, &index)?;
    }
    Ok(())
}

// ============================================================================
// Capes — owned capes come from the account profile; the active one can be
// switched or removed via the Minecraft Services API.
// ============================================================================

const PROFILE_API: &str = "https://api.minecraftservices.com/minecraft/profile";
const ACTIVE_CAPE_API: &str = "https://api.minecraftservices.com/minecraft/profile/capes/active";

#[derive(Deserialize)]
struct McProfile {
    #[serde(default)]
    capes: Vec<McCape>,
}

#[derive(Deserialize)]
struct McCape {
    id: String,
    state: String,
    url: String,
    #[serde(default)]
    alias: Option<String>,
}

/// A cape the player owns: `data_url` is a cropped front-face preview (for the
/// grid), `texture` is the full cape PNG (for the 3D model).
#[derive(Serialize)]
pub struct CapeEntry {
    id: String,
    name: String,
    active: bool,
    data_url: String,
    texture: String,
}

/// List the capes on the signed-in account (each with a preview + full texture).
#[tauri::command]
pub async fn get_capes() -> Result<Vec<CapeEntry>, String> {
    let profile = crate::auth::login_silent()
        .await
        .map_err(|e| format!("auth: {e}"))?
        .ok_or("You must be signed in to see your capes.")?;

    let client = reqwest::Client::builder()
        .user_agent("GPClient/0.1 (+launcher)")
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(PROFILE_API)
        .bearer_auth(&profile.access_token)
        .send()
        .await
        .map_err(|e| format!("fetch profile: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("fetch profile: HTTP {}", resp.status()));
    }
    let prof: McProfile = resp.json().await.map_err(|e| format!("parse profile: {e}"))?;

    let mut out = Vec::new();
    for cape in &prof.capes {
        // Skip capes whose texture won't load rather than failing the whole call.
        let Ok(resp) = client.get(&cape.url).send().await else {
            continue;
        };
        let Ok(bytes) = resp.bytes().await else {
            continue;
        };
        let Ok(preview) = crop_cape_front(&bytes) else {
            continue;
        };
        out.push(CapeEntry {
            id: cape.id.clone(),
            name: cape.alias.clone().unwrap_or_else(|| "Cape".to_string()),
            active: cape.state.eq_ignore_ascii_case("ACTIVE"),
            data_url: preview,
            texture: data_url(&bytes),
        });
    }
    Ok(out)
}

/// Crop a cape texture's front face and scale it up to a preview data URL.
fn crop_cape_front(bytes: &[u8]) -> Result<String, String> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| e.to_string())?
        .to_rgba8();
    let (w, h) = (img.width(), img.height());
    // The cape front lives at (1,1) size 10x16 on a base 64x32 texture; scale
    // those coordinates for HD capes.
    let sx = w as f32 / 64.0;
    let sy = h as f32 / 32.0;
    let front = imageops::crop_imm(
        &img,
        (sx).round() as u32,
        (sy).round() as u32,
        (10.0 * sx).round() as u32,
        (16.0 * sy).round() as u32,
    )
    .to_image();
    let scaled = imageops::resize(&front, 80, 128, imageops::FilterType::Nearest);

    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(scaled)
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| format!("encode png: {e}"))?;
    Ok(data_url(buf.get_ref()))
}

/// Set (or clear) the active cape. `cape` = a cape id to activate, or None /
/// "none" to hide the cape.
#[tauri::command]
pub async fn set_cape(cape: Option<String>) -> Result<(), String> {
    let profile = crate::auth::login_silent()
        .await
        .map_err(|e| format!("auth: {e}"))?
        .ok_or("You must be signed in to change your cape.")?;

    let client = reqwest::Client::builder()
        .user_agent("GPClient/0.1 (+launcher)")
        .build()
        .map_err(|e| e.to_string())?;

    let req = match cape {
        Some(id) if id != "none" && !id.is_empty() => client
            .put(ACTIVE_CAPE_API)
            .bearer_auth(&profile.access_token)
            .json(&serde_json::json!({ "capeId": id })),
        _ => client.delete(ACTIVE_CAPE_API).bearer_auth(&profile.access_token),
    };

    let resp = req.send().await.map_err(|e| format!("set cape: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Couldn't change cape (HTTP {status}). {body}"));
    }
    Ok(())
}
