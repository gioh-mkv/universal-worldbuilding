use std::fs;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, State};

use crate::domain::backup::{self, BackupInfo, IntegrityReport};
use crate::domain::entities::{self, Entity, NewEntity, UpdateEntity};
use crate::domain::entity_types::{self, EntityType, NewEntityType, UpdateEntityType};
use crate::domain::links::{self, EntityLinks};
use crate::domain::projects::{self, BackupSettings, ProjectInfo};
use crate::domain::relations::{
    self, NewRelation, NewRelationType, RelationType, RelationView, UpdateRelationType,
};
use crate::domain::search::{self, SearchHit, SearchQuery};
use crate::domain::transfer::{self, ExportResult, ImportSummary};
use crate::domain::universes::{self, Universe};
use crate::error::{AppError, Result};
use crate::state::AppState;

/// Projetos ficam em ~/Documentos/UWB/<projeto>/ (ou ~/UWB se não houver pasta Documentos).
fn base_dir(app: &AppHandle) -> Result<PathBuf> {
    let base = app
        .path()
        .document_dir()
        .or_else(|_| app.path().home_dir())
        .map_err(|e| AppError::msg(e.to_string()))?;
    Ok(base.join("UWB"))
}

// ---------- Projetos ----------

#[tauri::command]
pub fn list_projects(app: AppHandle) -> Result<Vec<ProjectInfo>> {
    projects::list(&base_dir(&app)?)
}

#[tauri::command]
pub fn create_project(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<ProjectInfo> {
    let base = base_dir(&app)?;
    fs::create_dir_all(&base)?;
    let (info, open) = projects::create(&base, &name)?;
    state.set(open)?;
    Ok(info)
}

#[tauri::command]
pub fn open_project(state: State<'_, AppState>, path: String) -> Result<ProjectInfo> {
    let (info, open) = projects::open(Path::new(&path))?;
    state.set(open)?;
    Ok(info)
}

#[tauri::command]
pub async fn duplicate_project(
    app: AppHandle,
    path: String,
    new_name: String,
) -> Result<ProjectInfo> {
    let base = base_dir(&app)?;
    projects::duplicate(&base, Path::new(&path), &new_name)
}

/// Apaga a pasta inteira do projeto, inclusive os backups que estão dentro dela.
#[tauri::command]
pub fn delete_project(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<()> {
    let base = base_dir(&app)?;
    let dir = projects::resolve(&base, Path::new(&path))?;
    if let Some(current) = state.current_dir()? {
        if fs::canonicalize(&current).ok().as_deref() == Some(dir.as_path()) {
            state.close()?; // fecha o banco antes de apagar os arquivos
        }
    }
    projects::remove(&dir)
}

// ---------- Backup ----------

#[tauri::command]
pub fn list_backups(state: State<'_, AppState>) -> Result<Vec<BackupInfo>> {
    state.with_project(|_, dir| backup::list(dir))
}

#[tauri::command]
pub async fn create_backup(
    state: State<'_, AppState>,
    label: Option<String>,
) -> Result<BackupInfo> {
    state.with_project(|conn, dir| backup::create(conn, dir, "manual", label.as_deref()))
}

#[tauri::command]
pub async fn verify_backup(state: State<'_, AppState>, id: String) -> Result<IntegrityReport> {
    state.with_project(|_, dir| backup::verify(dir, &id))
}

#[tauri::command]
pub fn delete_backup(state: State<'_, AppState>, id: String) -> Result<()> {
    state.with_project(|_, dir| backup::delete(dir, &id))
}

/// Restaura um backup. Antes de sobrescrever, guarda o estado atual como
/// backup "pre-restore". Se algo falhar, o banco atual continua valendo.
#[tauri::command]
pub async fn restore_backup(state: State<'_, AppState>, id: String) -> Result<ProjectInfo> {
    state.swap(|open| {
        let dir = open.dir.clone();
        let backup_db = backup::prepare_restore(&dir, &id)?;
        backup::create(
            &open.conn,
            &dir,
            "pre-restore",
            Some(&format!("Antes de restaurar {id}")),
        )?;
        drop(open); // fecha a conexão antes de trocar o arquivo
        backup::apply_restore(&dir, &backup_db)?;
        let manifest = projects::load_manifest(&dir)?;
        Ok(ProjectInfo {
            name: manifest.name,
            path: dir.to_string_lossy().into_owned(),
        })
    })
}

#[tauri::command]
pub async fn run_auto_backup(state: State<'_, AppState>) -> Result<Option<BackupInfo>> {
    state.with_project(|conn, dir| backup::auto_if_due(conn, dir))
}

#[tauri::command]
pub async fn check_project_integrity(state: State<'_, AppState>) -> Result<IntegrityReport> {
    state.with(backup::check_live)
}

#[tauri::command]
pub fn get_backup_settings(state: State<'_, AppState>) -> Result<BackupSettings> {
    state.with_project(|_, dir| Ok(projects::load_manifest(dir)?.backup))
}

#[tauri::command]
pub fn set_backup_settings(
    state: State<'_, AppState>,
    settings: BackupSettings,
) -> Result<BackupSettings> {
    if !(1..=720).contains(&settings.interval_hours) {
        return Err(AppError::msg("o intervalo deve ficar entre 1 e 720 horas"));
    }
    if !(1..=100).contains(&settings.keep_auto) {
        return Err(AppError::msg(
            "a quantidade de backups automáticos deve ficar entre 1 e 100",
        ));
    }
    state.with_project(|_, dir| {
        let mut manifest = projects::load_manifest(dir)?;
        manifest.backup = settings.clone();
        projects::save_manifest(dir, &manifest)?;
        Ok(())
    })?;
    Ok(settings)
}

// ---------- Exportação e importação ----------

#[tauri::command]
pub async fn export_universe_json(
    state: State<'_, AppState>,
    universe_id: String,
    path: String,
) -> Result<ExportResult> {
    state.with(|c| transfer::export_json(c, &universe_id, Path::new(&path)))
}

#[tauri::command]
pub async fn export_universe_markdown(
    state: State<'_, AppState>,
    universe_id: String,
    dir: String,
) -> Result<ExportResult> {
    state.with(|c| transfer::export_markdown(c, &universe_id, Path::new(&dir)))
}

#[tauri::command]
pub async fn import_universe_json(
    state: State<'_, AppState>,
    path: String,
    new_name: Option<String>,
) -> Result<ImportSummary> {
    // A leitura do arquivo acontece antes de pegar o banco.
    let data = transfer::parse_export(Path::new(&path))?;
    state.with(|c| transfer::import_universe(c, data, new_name.as_deref()))
}

// ---------- Universos ----------

#[tauri::command]
pub fn create_universe(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
) -> Result<Universe> {
    state.with(|c| universes::create(c, &name, description.as_deref()))
}

#[tauri::command]
pub fn list_universes(state: State<'_, AppState>) -> Result<Vec<Universe>> {
    state.with(universes::list)
}

#[tauri::command]
pub fn update_universe(
    state: State<'_, AppState>,
    id: String,
    name: String,
    description: Option<String>,
) -> Result<Universe> {
    state.with(|c| universes::update(c, &id, &name, description.as_deref()))
}

#[tauri::command]
pub fn delete_universe(state: State<'_, AppState>, id: String) -> Result<()> {
    state.with(|c| universes::delete(c, &id))
}

// ---------- Tipos de entidade ----------

#[tauri::command]
pub fn create_entity_type(
    state: State<'_, AppState>,
    input: NewEntityType,
) -> Result<EntityType> {
    state.with(|c| entity_types::create(c, input))
}

#[tauri::command]
pub fn update_entity_type(
    state: State<'_, AppState>,
    input: UpdateEntityType,
) -> Result<EntityType> {
    state.with(|c| entity_types::update(c, input))
}

#[tauri::command]
pub fn delete_entity_type(state: State<'_, AppState>, id: String) -> Result<()> {
    state.with(|c| entity_types::delete(c, &id))
}

#[tauri::command]
pub fn list_entity_types(
    state: State<'_, AppState>,
    universe_id: String,
) -> Result<Vec<EntityType>> {
    state.with(|c| entity_types::list(c, &universe_id))
}

// ---------- Entidades ----------

#[tauri::command]
pub fn create_entity(state: State<'_, AppState>, input: NewEntity) -> Result<Entity> {
    state.with(|c| entities::create(c, input))
}

#[tauri::command]
pub fn update_entity(state: State<'_, AppState>, input: UpdateEntity) -> Result<Entity> {
    state.with(|c| entities::update(c, input))
}

#[tauri::command]
pub fn delete_entity(state: State<'_, AppState>, id: String) -> Result<()> {
    state.with(|c| entities::delete(c, &id))
}

#[tauri::command]
pub fn list_entities(
    state: State<'_, AppState>,
    universe_id: String,
    type_id: Option<String>,
) -> Result<Vec<Entity>> {
    state.with(|c| entities::list(c, &universe_id, type_id.as_deref()))
}

#[tauri::command]
pub fn get_entity_links(state: State<'_, AppState>, entity_id: String) -> Result<EntityLinks> {
    state.with(|c| links::for_entity(c, &entity_id))
}

// ---------- Relações ----------

#[tauri::command]
pub fn create_relation_type(
    state: State<'_, AppState>,
    input: NewRelationType,
) -> Result<RelationType> {
    state.with(|c| relations::create_type(c, input))
}

#[tauri::command]
pub fn update_relation_type(
    state: State<'_, AppState>,
    input: UpdateRelationType,
) -> Result<RelationType> {
    state.with(|c| relations::update_type(c, input))
}

#[tauri::command]
pub fn delete_relation_type(state: State<'_, AppState>, id: String) -> Result<()> {
    state.with(|c| relations::delete_type(c, &id))
}

#[tauri::command]
pub fn list_relation_types(
    state: State<'_, AppState>,
    universe_id: String,
) -> Result<Vec<RelationType>> {
    state.with(|c| relations::list_types(c, &universe_id))
}

#[tauri::command]
pub fn create_relation(state: State<'_, AppState>, input: NewRelation) -> Result<String> {
    state.with(|c| relations::create(c, input))
}

#[tauri::command]
pub fn delete_relation(state: State<'_, AppState>, id: String) -> Result<()> {
    state.with(|c| relations::delete(c, &id))
}

#[tauri::command]
pub fn list_entity_relations(
    state: State<'_, AppState>,
    entity_id: String,
) -> Result<Vec<RelationView>> {
    state.with(|c| relations::list_for_entity(c, &entity_id))
}

// ---------- Pesquisa ----------

#[tauri::command]
pub fn search(state: State<'_, AppState>, query: SearchQuery) -> Result<Vec<SearchHit>> {
    state.with(|c| search::search(c, query))
}