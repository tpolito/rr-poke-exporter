mod charmap;
mod data;
mod parser;
mod settings;

use tauri::AppHandle;

#[tauri::command]
fn parse_sav_file(app: AppHandle, path: String) -> Result<Vec<parser::Pokemon>, String> {
    settings::set_saved_path(&app, &path)?;
    parser::parse_sav(&path)
}

#[tauri::command]
fn get_saved_path(app: AppHandle) -> Option<String> {
    settings::get_saved_path(&app)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![parse_sav_file, get_saved_path])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
