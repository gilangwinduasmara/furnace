// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use furnace_core::services;

#[tauri::command]
fn furnace_status() -> String {
    // For now, just call the status function (which prints to stdout)
    // In the future, refactor to return a structured status
    services::status();
    "Status checked (see logs)".to_string()
}

#[tauri::command]
fn recipe_list() -> Vec<furnace_core::recipe::Recipe> {
    furnace_core::recipe::get_recipes()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![furnace_status, recipe_list])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
