mod auth;
mod brand;
mod fabric;
mod installations;
mod launch;
mod mods;
mod mojang;
mod profile_import;
mod settings;
mod skin;
mod updates;

use tauri::Manager;

/// Returns the brand strings to the frontend. The frontend also imports
/// brand.json directly for static rendering; this command exists so native
/// code and the UI provably share one source of truth.
#[tauri::command]
fn get_brand() -> brand::Brand {
    brand::brand()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // The window title is defined as a placeholder in tauri.conf.json;
            // override it at startup from the single brand source of truth.
            let title = brand::brand().window_title;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title(&title);
            }
            // Make sure the base installations tree exists on startup. A failure
            // here shouldn't stop the app from opening, so just log it.
            if let Err(e) = installations::ensure_base() {
                eprintln!("warning: could not create installations base tree: {e}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_brand,
            auth::auth_login,
            auth::auth_login_silent,
            auth::auth_logout,
            auth::auth_status,
            installations::installations_root,
            installations::create_installation,
            installations::list_installations_cmd,
            installations::open_installations_folder,
            installations::open_installation_folder,
            installations::update_installation,
            installations::delete_installation,
            launch::launch_version,
            mods::sync_mods,
            mods::list_mod_versions,
            mods::get_version_mods,
            mods::set_optional_mods,
            skin::get_skin_face,
            skin::get_skin,
            skin::get_player_textures,
            skin::list_skins,
            skin::rename_skin,
            skin::apply_skin,
            skin::import_skin,
            skin::update_skin,
            skin::replace_skin_file,
            skin::delete_skin,
            skin::get_capes,
            skin::set_cape,
            updates::check_for_update,
            updates::install_update,
            updates::release_notes,
            settings::get_settings,
            settings::set_settings,
            profile_import::list_source_profiles,
            profile_import::import_profile_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
