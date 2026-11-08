use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::db;
use crate::error::{AppError, Result};
use crate::state::OpenProject;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BackupSettings {
    pub auto_enabled: bool,
    pub interval_hours: u32,
    /// Quantos backups automáticos manter (manuais nunca são apagados sozinhos).
    pub keep_auto: u32,
}

impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            auto_enabled: true,
            interval_hours: 24,
            keep_auto: 10,
        }
    }
}

/// Conteúdo de project.json.
#[derive(Serialize, Deserialize, Clone)]
pub struct Manifest {
    pub name: String,
    #[serde(default)]
    pub backup: BackupSettings,
}

pub fn load_manifest(dir: &Path) -> Result<Manifest> {
    let raw = fs::read_to_string(dir.join("project.json"))
        .map_err(|_| AppError::msg("pasta inválida: project.json não encontrado"))?;
    Ok(serde_json::from_str(&raw)?)
}

pub fn save_manifest(dir: &Path, m: &Manifest) -> Result<()> {
    fs::write(dir.join("project.json"), serde_json::to_string_pretty(m)?)?;
    Ok(())
}

fn slugify(name: &str) -> String {
    let s: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    s.split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn info_of(name: &str, dir: &Path) -> ProjectInfo {
    ProjectInfo {
        name: name.to_string(),
        path: dir.to_string_lossy().into_owned(),
    }
}

/// Cria <base>/<slug>/ com project.json, project.db, assets/ e backups/.
pub fn create(base: &Path, name: &str) -> Result<(ProjectInfo, OpenProject)> {
    let name = name.trim();
    let slug = slugify(name);
    if slug.is_empty() {
        return Err(AppError::msg("nome de projeto inválido"));
    }
    let dir = base.join(slug);
    if dir.exists() {
        return Err(AppError::msg("já existe um projeto com esse nome"));
    }
    fs::create_dir_all(dir.join("assets"))?;
    fs::create_dir_all(dir.join("backups"))?;
    save_manifest(
        &dir,
        &Manifest {
            name: name.to_string(),
            backup: BackupSettings::default(),
        },
    )?;
    let conn = db::open(&dir.join("project.db"))?;
    Ok((info_of(name, &dir), OpenProject { conn, dir }))
}

pub fn open(dir: &Path) -> Result<(ProjectInfo, OpenProject)> {
    let manifest = load_manifest(dir)?;
    let conn = db::open(&dir.join("project.db"))?;
    Ok((
        info_of(&manifest.name, dir),
        OpenProject {
            conn,
            dir: dir.to_path_buf(),
        },
    ))
}

pub fn list(base: &Path) -> Result<Vec<ProjectInfo>> {
    let mut out = Vec::new();
    if !base.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(base)? {
        let path = entry?.path();
        if let Ok(m) = load_manifest(&path) {
            out.push(info_of(&m.name, &path));
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

/// Garante que `dir` é um projeto diretamente dentro de `base` (evita apagar/copiar
/// pastas arbitrárias). Devolve o caminho canônico.
pub fn resolve(base: &Path, dir: &Path) -> Result<PathBuf> {
    let canon_base = fs::canonicalize(base)
        .map_err(|_| AppError::msg("pasta de projetos não encontrada"))?;
    let canon = fs::canonicalize(dir).map_err(|_| AppError::msg("projeto não encontrado"))?;
    if canon.parent() != Some(canon_base.as_path()) || !canon.join("project.json").is_file() {
        return Err(AppError::msg("caminho de projeto inválido"));
    }
    Ok(canon)
}

/// Apaga a pasta inteira do projeto (banco, arquivos e backups). Definitivo.
pub fn remove(dir: &Path) -> Result<()> {
    if !dir.join("project.json").is_file() {
        return Err(AppError::msg("caminho de projeto inválido"));
    }
    fs::remove_dir_all(dir)?;
    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    if !src.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Duplica um projeto (banco consistente + arquivos). Os backups não são copiados.
pub fn duplicate(base: &Path, src: &Path, new_name: &str) -> Result<ProjectInfo> {
    let src = resolve(base, src)?;
    let new_name = new_name.trim();
    let slug = slugify(new_name);
    if slug.is_empty() {
        return Err(AppError::msg("nome de projeto inválido"));
    }
    let dest = base.join(slug);
    if dest.exists() {
        return Err(AppError::msg("já existe um projeto com esse nome"));
    }
    let source_manifest = load_manifest(&src)?;
    fs::create_dir_all(dest.join("backups"))?;

    let built = (|| -> Result<()> {
        let src_conn = db::open(&src.join("project.db"))?;
        let target = dest.join("project.db").to_string_lossy().into_owned();
        src_conn.execute("VACUUM INTO ?1", [target.as_str()])?;
        drop(src_conn);
        copy_dir(&src.join("assets"), &dest.join("assets"))?;
        save_manifest(
            &dest,
            &Manifest {
                name: new_name.to_string(),
                backup: source_manifest.backup.clone(),
            },
        )?;
        Ok(())
    })();

    if let Err(e) = built {
        let _ = fs::remove_dir_all(&dest);
        return Err(e);
    }
    Ok(info_of(new_name, &dest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::universes;

    fn tmp() -> PathBuf {
        let p = std::env::temp_dir().join(format!("uwb-test-{}", db::new_id()));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn duplicar_projeto_copia_banco_e_assets() {
        let base = tmp();
        let (info, opened) = create(&base, "Original").unwrap();
        universes::create(&opened.conn, "U", None).unwrap();
        fs::write(base.join("original/assets/a.txt"), "x").unwrap();
        drop(opened);

        let copy = duplicate(&base, Path::new(&info.path), "Cópia").unwrap();
        let copy_dir_path = PathBuf::from(&copy.path);

        let (copy_info, copy_open) = open(&copy_dir_path).unwrap();
        assert_eq!(copy_info.name, "Cópia");
        assert_eq!(universes::list(&copy_open.conn).unwrap().len(), 1);
        assert!(copy_dir_path.join("assets/a.txt").is_file());
        assert!(copy_dir_path.join("backups").is_dir());

        drop(copy_open);
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn excluir_so_dentro_da_base() {
        let base = tmp();
        let outside = tmp();
        fs::write(outside.join("project.json"), "{\"name\":\"X\"}").unwrap();

        // Pasta fora da base: recusada.
        assert!(resolve(&base, &outside).is_err());

        // Projeto dentro da base: aceito e removido.
        let (info, opened) = create(&base, "Dentro").unwrap();
        drop(opened);
        let dir = resolve(&base, Path::new(&info.path)).unwrap();
        remove(&dir).unwrap();
        assert!(!dir.exists());

        fs::remove_dir_all(&base).ok();
        fs::remove_dir_all(&outside).ok();
    }
}