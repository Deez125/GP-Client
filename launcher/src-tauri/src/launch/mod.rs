//! Step 6/7: assemble the launch command and spawn Minecraft, optionally with
//! the Fabric loader and a mod sync beforehand.
//!
//! Pipeline:
//!   1. (optional) sync GP-managed mods into the instance `mods/` folder.
//!   2. download/verify the vanilla game files (client, libraries, assets).
//!   3. (optional) resolve Fabric: extra libraries + KnotClient main class.
//!   4. find a matching Java, expand the JVM/game argument templates, spawn.
//!
//! For now it launches with a placeholder OFFLINE profile so we can verify the
//! pipeline before Microsoft approves the Minecraft API. Real auth drops in
//! later by replacing the auth fields below.

mod java;

use std::collections::HashMap;

use tauri::{AppHandle, Manager};
use tokio::process::Command;

use crate::brand;
use crate::mojang::{self, rules};
use crate::{fabric, mods};

const OFFLINE_NAME: &str = "Player";
const OFFLINE_UUID: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";

fn classpath_separator() -> &'static str {
    if cfg!(windows) {
        ";"
    } else {
        ":"
    }
}

fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("GPClient/0.1 (+launcher)")
        .build()
        .map_err(|e| e.to_string())
}

/// Prepare files, resolve Java, build the command, and spawn the game.
pub async fn launch(
    app: &AppHandle,
    version_id: &str,
    use_fabric: bool,
    sync_mods: bool,
    server: Option<&str>,
) -> Result<(), String> {
    // 1. Sync GP-managed mods first (non-destructive).
    if sync_mods {
        mods::sync(app, version_id).await?;
    }

    // 2. Vanilla game files.
    let prepared = mojang::prepare(app, version_id).await?;
    let version_name = prepared.details.id.clone();

    // 3. Optional Fabric loader.
    let mut classpath: Vec<std::path::PathBuf> = Vec::new();
    let mut main_class = prepared.details.main_class.clone();
    let mut extra_jvm: Vec<String> = Vec::new();
    let mut extra_game: Vec<String> = Vec::new();

    if use_fabric {
        let client = http()?;
        let fab = fabric::prepare(app, &client, &prepared.libraries_dir, &version_name).await?;
        mojang::emit_progress(
            app,
            "fabric",
            1,
            1,
            &format!("Fabric {} ready", fab.loader_version),
        );
        classpath.extend(fab.libraries);
        main_class = fab.main_class;
        extra_jvm = fab.jvm_args;
        extra_game = fab.game_args;
    }

    // Final classpath: [fabric libs] + vanilla libs + client jar (last).
    classpath.extend(prepared.classpath.iter().cloned());
    classpath.push(prepared.client_jar.clone());

    let classpath_str = classpath
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(classpath_separator());

    // Placeholder substitution table used by both arg lists.
    let mut vars: HashMap<String, String> = HashMap::new();
    vars.insert("natives_directory".into(), path_str(&prepared.natives_dir));
    vars.insert("launcher_name".into(), brand::brand().app_name);
    vars.insert("launcher_version".into(), env!("CARGO_PKG_VERSION").to_string());
    vars.insert("classpath".into(), classpath_str);
    vars.insert("classpath_separator".into(), classpath_separator().into());
    vars.insert("library_directory".into(), path_str(&prepared.libraries_dir));
    vars.insert("version_name".into(), version_name.clone());
    vars.insert("game_directory".into(), path_str(&prepared.game_dir));
    vars.insert("assets_root".into(), path_str(&prepared.assets_dir));
    vars.insert(
        "assets_index_name".into(),
        prepared.details.asset_index.id.clone(),
    );
    vars.insert("version_type".into(), prepared.details.version_type.clone());

    // Use the signed-in account if there is one (fresh token via silent
    // refresh); otherwise fall back to an offline profile. A failure here
    // (e.g. no network) degrades gracefully to offline rather than blocking.
    let session = crate::auth::login_silent().await.ok().flatten();
    match &session {
        Some(profile) => {
            mojang::emit_progress(
                app,
                "auth",
                1,
                1,
                &format!("Signed in as {}", profile.username),
            );
            vars.insert("auth_player_name".into(), profile.username.clone());
            vars.insert("auth_uuid".into(), profile.uuid.clone());
            vars.insert("auth_access_token".into(), profile.access_token.clone());
            vars.insert("user_type".into(), "msa".into());
        }
        None => {
            vars.insert("auth_player_name".into(), OFFLINE_NAME.into());
            vars.insert("auth_uuid".into(), OFFLINE_UUID.into());
            vars.insert("auth_access_token".into(), "0".into());
            vars.insert("user_type".into(), "msa".into());
        }
    }
    vars.insert("auth_xuid".into(), String::new());
    vars.insert("clientid".into(), String::new());

    let features: HashMap<String, bool> = HashMap::new();

    // Per-installation memory + extra JVM args (launcher-only settings).
    let instance = crate::installations::instance_meta(&version_name);

    // Vanilla templated args + Fabric's extra (literal) args, all expanded.
    let mut jvm_args = rules::resolve_args(&prepared.details.arguments.jvm, &features, &vars);
    jvm_args.extend(extra_jvm.iter().map(|a| rules::expand(a, &vars)));
    // Memory: -Xmx drives the slider value; a modest -Xms.
    jvm_args.insert(0, format!("-Xmx{}m", instance.ram_mb));
    jvm_args.insert(1, format!("-Xms{}m", instance.ram_mb.min(1024)));
    // User's extra JVM args (whitespace-separated).
    for arg in instance.jvm_args.split_whitespace() {
        jvm_args.push(arg.to_string());
    }

    let mut game_args = rules::resolve_args(&prepared.details.arguments.game, &features, &vars);
    game_args.extend(extra_game.iter().map(|a| rules::expand(a, &vars)));

    // Quick join: boot straight into a server (Minecraft 1.20+ quick-play).
    if let Some(addr) = server.map(str::trim).filter(|a| !a.is_empty()) {
        game_args.push("--quickPlayMultiplayer".to_string());
        game_args.push(addr.to_string());
        // Temporary: the current Iris (a 26.1.1 build) crashes on pause when a
        // shader is active on 26.1.2. Quick-join is a one-tap action with no
        // chance to toggle shaders first, so force them off until we ship a
        // 26.1.2-compatible Iris/shader combo.
        disable_iris_shaders(&prepared.game_dir);
    }

    // Resolve Java: a matching system JDK, or download Mojang's bundled runtime.
    let required = prepared.details.java_version.major_version;
    let component = prepared.details.java_version.component.clone();
    let java = java::ensure_java(app, &component, required).await?;

    // On Windows, launch with `javaw.exe` (the windowless Java) so no console
    // pops up; fall back to the resolved java if javaw isn't beside it.
    let mut exe = java.clone();
    #[cfg(windows)]
    {
        let javaw = java.with_file_name("javaw.exe");
        if javaw.exists() {
            exe = javaw;
        }
    }

    // java[w] <jvm args> <mainClass> <game args>
    let mut cmd = Command::new(&exe);
    cmd.current_dir(&prepared.game_dir);
    cmd.args(&jvm_args);
    cmd.arg(&main_class);
    cmd.args(&game_args);

    // Belt-and-suspenders: also suppress any console window on Windows.
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let log_path = prepared.game_dir.join("gpclient-latest.log");
    if let Ok(log) = std::fs::File::create(&log_path) {
        if let Ok(err) = log.try_clone() {
            cmd.stdout(log);
            cmd.stderr(err);
        }
    }

    mojang::emit_progress(app, "launch", 1, 1, "Starting Minecraft…");
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to start Java ({}): {e}", java.display()))?;

    // Apply the user's launch-behavior setting now that the game is up.
    let settings = crate::settings::load();
    match settings.launch_behavior.as_str() {
        "minimize" => {
            if let Some(w) = main_window(app) {
                let _ = w.minimize();
            }
        }
        "close" => {
            // Quit the launcher; the spawned game keeps running independently.
            app.exit(0);
            return Ok(());
        }
        _ => {} // "keep": leave the window as-is.
    }

    // Keep the caller (and thus the Play button) "busy" for the whole game
    // session — re-enable only when Minecraft exits.
    mojang::emit_progress(app, "running", 1, 1, "Minecraft is running");
    let status = child
        .wait()
        .await
        .map_err(|e| format!("error waiting for Minecraft: {e}"))?;

    // Bring the launcher back when the game exits, if the user wants it.
    if settings.reopen_on_close {
        if let Some(w) = main_window(app) {
            let _ = w.unminimize();
            let _ = w.show();
            // Windows blocks a background app from grabbing the foreground via
            // set_focus alone; briefly forcing always-on-top pulls it to front.
            let _ = w.set_always_on_top(true);
            let _ = w.set_focus();
            let _ = w.set_always_on_top(false);
        }
    }

    if !status.success() {
        // Non-zero usually means a crash, not a normal quit — point at the log.
        return Err(format!(
            "Minecraft exited with an error ({status}). See {}",
            log_path.display()
        ));
    }
    Ok(())
}

fn path_str(p: &std::path::Path) -> String {
    p.to_string_lossy().into_owned()
}

/// Force Iris shaders off for this instance by setting `enableShaders=false`
/// in `config/iris.properties`, preserving every other line. Creates the file
/// (and `config/`) if it's not there yet. Best-effort: a failure just means the
/// game launches with whatever shader state it had.
fn disable_iris_shaders(game_dir: &std::path::Path) {
    let path = game_dir.join("config").join("iris.properties");
    let mut lines: Vec<String> = std::fs::read_to_string(&path)
        .map(|s| s.lines().map(str::to_string).collect())
        .unwrap_or_default();

    let mut found = false;
    for line in lines.iter_mut() {
        // Only an active (non-comment) `enableShaders=...` line.
        if line.trim_start().starts_with("enableShaders") {
            *line = "enableShaders=false".to_string();
            found = true;
        }
    }
    if !found {
        lines.push("enableShaders=false".to_string());
    }

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, lines.join("\n") + "\n");
}

/// The launcher's window. Prefers the conventional "main" label but falls back
/// to whatever window exists, so window control works regardless of label.
fn main_window(app: &AppHandle) -> Option<tauri::WebviewWindow> {
    app.get_webview_window("main")
        .or_else(|| app.webview_windows().into_values().next())
}

// --- Tauri command ----------------------------------------------------------

/// `fabric` and `sync_mods` default to true when omitted by the caller.
/// `server`, when given, boots straight into that server (quick join).
#[tauri::command]
pub async fn launch_version(
    app: AppHandle,
    version: String,
    fabric: Option<bool>,
    sync_mods: Option<bool>,
    server: Option<String>,
) -> Result<(), String> {
    launch(
        &app,
        &version,
        fabric.unwrap_or(true),
        sync_mods.unwrap_or(true),
        server.as_deref(),
    )
    .await
}
