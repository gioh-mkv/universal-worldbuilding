mod commands;
mod db;
mod domain;
mod error;
mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_projects,
            commands::create_project,
            commands::open_project,
            commands::duplicate_project,
            commands::delete_project,
            commands::list_backups,
            commands::create_backup,
            commands::verify_backup,
            commands::delete_backup,
            commands::restore_backup,
            commands::run_auto_backup,
            commands::check_project_integrity,
            commands::get_backup_settings,
            commands::set_backup_settings,
            commands::export_universe_json,
            commands::export_universe_markdown,
            commands::import_universe_json,
            commands::create_universe,
            commands::list_universes,
            commands::update_universe,
            commands::delete_universe,
            commands::create_entity_type,
            commands::update_entity_type,
            commands::delete_entity_type,
            commands::list_entity_types,
            commands::create_entity,
            commands::update_entity,
            commands::delete_entity,
            commands::list_entities,
            commands::get_entity_links,
            commands::create_relation_type,
            commands::update_relation_type,
            commands::delete_relation_type,
            commands::list_relation_types,
            commands::create_relation,
            commands::delete_relation,
            commands::list_entity_relations,
            commands::search,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o UWB");
}