use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::projects;
use crate::db;
use crate::error::{AppError, Result};

const MANIFEST_FILE: &str = "manifest.json";
const DB_FILE: &str = "project.db";
const KINDS: [&str; 3] = ["manual", "auto", "pre-restore"];

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub universes: i64,
    pub entities: i64,
    pub entity_types: i64,
    pub relations: i64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackupManifest {
    pub format_version: u32,
    /// "manual" | "auto" | "pre-restore"
    pub kind: String,
    pub label: Option<String>,
    /// UTC, ISO-8601.
    pub created_at: String,
    pub project_name: String,
    pub schema_version: u32,
    pub app_version: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub counts: Counts,
    /// Arquivos anexados ainda não existem; quando existirem, passam a entrar no backup.
    pub includes_assets: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub id: String,
    #[serde(flatten)]
    pub manifest: BackupManifest,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    pub ok: bool,
    pub checks: Vec<Check>,
}

impl IntegrityReport {
    fn failures(&self) -> String {
        self.checks
            .iter()
            .filter(|c| !c.ok)
            .map(|c| format!("{}: {}", c.name, c.detail))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

fn report(checks: Vec<Check>) -> IntegrityReport {
    IntegrityReport {
        ok: checks.iter().all(|c| c.ok),
        checks,
    }
}

// ---------- utilidades ----------

pub fn backups_dir(project_dir: &Path) -> PathBuf {
    project_dir.join("backups")
}

/// Ids viram nomes de pasta: só letras, números, "-" e "_" (sem "..", "/" ou ".").
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn backup_path(project_dir: &Path, id: &str) -> Result<PathBuf> {
    if !valid_id(id) {
        return Err(AppError::msg("identificador de backup inválido"));
    }
    Ok(backups_dir(project_dir).join(id))
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut f = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn counts(conn: &Connection) -> Result<Counts> {
    let one = |table: &str| -> rusqlite::Result<i64> {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
    };
    Ok(Counts {
        universes: one("universes")?,
        entities: one("entities")?,
        entity_types: one("entity_types")?,
        relations: one("relations")?,
    })
}

fn manifest_of(dir: &Path) -> Result<BackupManifest> {
    let raw = fs::read_to_string(dir.join(MANIFEST_FILE))
        .map_err(|_| AppError::msg("backup sem manifesto (manifest.json)"))?;
    Ok(serde_json::from_str(&raw)?)
}

// ---------- verificação ----------

fn run_checks(conn: &Connection) -> Result<Vec<Check>> {
    let mut checks = Vec::new();

    let lines: Vec<String> = {
        let mut st = conn.prepare("PRAGMA integrity_check")?;
        let rows = st.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let ok = lines.len() == 1 && lines[0] == "ok";
    checks.push(Check {
        name: "Estrutura do banco (integrity_check)".into(),
        ok,
        detail: if ok {
            "ok".into()
        } else {
            lines.iter().take(5).cloned().collect::<Vec<_>>().join("; ")
        },
    });

    let violations = {
        let mut st = conn.prepare("PRAGMA foreign_key_check")?;
        let n = st.query_map([], |_| Ok(()))?.count();
        n
    };
    checks.push(Check {
        name: "Referências entre tabelas (foreign_key_check)".into(),
        ok: violations == 0,
        detail: if violations == 0 {
            "ok".into()
        } else {
            format!("{violations} violação(ões)")
        },
    });

    Ok(checks)
}

/// Verifica o banco ABERTO do projeto.
pub fn check_live(conn: &Connection) -> Result<IntegrityReport> {
    Ok(report(run_checks(conn)?))
}

/// Verifica um backup: checksum + abertura somente leitura + checks do SQLite.
pub fn verify(project_dir: &Path, id: &str) -> Result<IntegrityReport> {
    let dir = backup_path(project_dir, id)?;
    let manifest = manifest_of(&dir)?;
    let db_path = dir.join(DB_FILE);
    let mut checks = Vec::new();

    if !db_path.is_file() {
        checks.push(Check {
            name: "Arquivo do banco".into(),
            ok: false,
            detail: "project.db não encontrado no backup".into(),
        });
        return Ok(report(checks));
    }

    let sha_ok = sha256_file(&db_path)? == manifest.sha256;
    checks.push(Check {
        name: "Checksum SHA-256".into(),
        ok: sha_ok,
        detail: if sha_ok {
            "confere com o manifesto".into()
        } else {
            "o arquivo foi alterado ou está corrompido".into()
        },
    });

    match Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(c) => match run_checks(&c) {
            Ok(v) => checks.extend(v),
            Err(e) => checks.push(Check {
                name: "Leitura do banco".into(),
                ok: false,
                detail: e.to_string(),
            }),
        },
        Err(e) => checks.push(Check {
            name: "Abertura do banco".into(),
            ok: false,
            detail: e.to_string(),
        }),
    }
    Ok(report(checks))
}

// ---------- criar / listar / excluir ----------

pub fn create(
    conn: &Connection,
    project_dir: &Path,
    kind: &str,
    label: Option<&str>,
) -> Result<BackupInfo> {
    if !KINDS.contains(&kind) {
        return Err(AppError::msg("tipo de backup inválido"));
    }
    let project_name = projects::load_manifest(project_dir)?.name;
    let (created_at, stamp): (String, String) = conn.query_row(
        "SELECT strftime('%Y-%m-%dT%H:%M:%SZ','now'), strftime('%Y%m%d-%H%M%S','now')",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;

    let root = backups_dir(project_dir);
    fs::create_dir_all(&root)?;
    let mut id = format!("{stamp}-{kind}");
    let mut n = 2;
    while root.join(&id).exists() {
        id = format!("{stamp}-{kind}-{n}");
        n += 1;
    }
    let tmp = root.join(format!(".tmp-{id}"));
    if tmp.exists() {
        fs::remove_dir_all(&tmp)?;
    }
    fs::create_dir_all(&tmp)?;

    let label: Option<String> = label
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(80).collect());

    let built = (|| -> Result<BackupManifest> {
        let db_path = tmp.join(DB_FILE);
        let target = db_path.to_string_lossy().into_owned();
        // Cópia consistente do banco, mesmo com o app em uso.
        conn.execute("VACUUM INTO ?1", [target.as_str()])?;

        let copy = Connection::open(&db_path)?;
        // Deixa o arquivo do backup em modo "journal" comum (sem arquivos -wal/-shm).
        copy.pragma_update(None, "journal_mode", "DELETE")?;
        let rep = report(run_checks(&copy)?);
        if !rep.ok {
            return Err(AppError::msg(format!(
                "o backup gerado não passou na verificação: {}",
                rep.failures()
            )));
        }
        let schema_version: usize = copy.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let counts = counts(&copy)?;
        drop(copy);

        let manifest = BackupManifest {
            format_version: 1,
            kind: kind.to_string(),
            label: label.clone(),
            created_at: created_at.clone(),
            project_name: project_name.clone(),
            schema_version: schema_version as u32,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            size_bytes: fs::metadata(&db_path)?.len(),
            sha256: sha256_file(&db_path)?,
            counts,
            includes_assets: false,
        };
        fs::write(
            tmp.join(MANIFEST_FILE),
            serde_json::to_string_pretty(&manifest)?,
        )?;
        Ok(manifest)
    })();

    match built {
        Ok(manifest) => {
            fs::rename(&tmp, root.join(&id))?;
            Ok(BackupInfo { id, manifest })
        }
        Err(e) => {
            let _ = fs::remove_dir_all(&tmp);
            Err(e)
        }
    }
}

/// Do mais novo para o mais antigo.
pub fn list(project_dir: &Path) -> Result<Vec<BackupInfo>> {
    let root = backups_dir(project_dir);
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(&root)? {
        let path = entry?.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !valid_id(name) {
            continue; // ignora pastas temporárias (".tmp-…") e estranhas
        }
        if let Ok(manifest) = manifest_of(&path) {
            out.push(BackupInfo {
                id: name.to_string(),
                manifest,
            });
        }
    }
    out.sort_by(|a, b| {
        b.manifest
            .created_at
            .cmp(&a.manifest.created_at)
            .then(b.id.cmp(&a.id))
    });
    Ok(out)
}

pub fn delete(project_dir: &Path, id: &str) -> Result<()> {
    let dir = backup_path(project_dir, id)?;
    if !dir.is_dir() {
        return Err(AppError::msg("backup não encontrado"));
    }
    fs::remove_dir_all(dir)?;
    Ok(())
}

/// Mantém só os `keep` backups automáticos mais recentes.
pub fn prune_auto(project_dir: &Path, keep: u32) -> Result<()> {
    let keep = keep.max(1) as usize;
    let autos: Vec<BackupInfo> = list(project_dir)?
        .into_iter()
        .filter(|b| b.manifest.kind == "auto")
        .collect();
    for old in autos.iter().skip(keep) {
        delete(project_dir, &old.id)?;
    }
    Ok(())
}

/// Cria um backup automático se estiver na hora (ou se ainda não houver nenhum).
pub fn auto_if_due(conn: &Connection, project_dir: &Path) -> Result<Option<BackupInfo>> {
    let settings = projects::load_manifest(project_dir)?.backup;
    if !settings.auto_enabled {
        return Ok(None);
    }
    let last = list(project_dir)?
        .into_iter()
        .find(|b| b.manifest.kind == "auto" || b.manifest.kind == "manual");

    if let Some(last) = last {
        let elapsed_hours: Option<f64> = conn.query_row(
            "SELECT (julianday('now') - julianday(?1)) * 24.0",
            [&last.manifest.created_at],
            |r| r.get(0),
        )?;
        if let Some(h) = elapsed_hours {
            if h < settings.interval_hours as f64 {
                return Ok(None);
            }
        }
    }

    let info = create(conn, project_dir, "auto", None)?;
    prune_auto(project_dir, settings.keep_auto)?;
    Ok(Some(info))
}

// ---------- restauração ----------

/// Valida o backup e devolve o caminho do banco a restaurar.
pub fn prepare_restore(project_dir: &Path, id: &str) -> Result<PathBuf> {
    let dir = backup_path(project_dir, id)?;
    let manifest = manifest_of(&dir)?;
    if manifest.schema_version as usize > db::SCHEMA_VERSION {
        return Err(AppError::msg(
            "este backup foi criado por uma versão mais nova do UWB; atualize o aplicativo",
        ));
    }
    let rep = verify(project_dir, id)?;
    if !rep.ok {
        return Err(AppError::msg(format!(
            "o backup não passou na verificação; restauração cancelada ({})",
            rep.failures()
        )));
    }
    Ok(dir.join(DB_FILE))
}

/// Substitui project.db pelo backup. O banco atual precisa estar FECHADO.
/// A cópia é feita para um arquivo temporário e só então trocada (rename atômico).
pub fn apply_restore(project_dir: &Path, backup_db: &Path) -> Result<()> {
    let live = project_dir.join(DB_FILE);
    let staged = project_dir.join("project.db.restoring");
    fs::copy(backup_db, &staged)?;
    for suffix in ["-wal", "-shm"] {
        let side = project_dir.join(format!("{DB_FILE}{suffix}"));
        if side.exists() {
            fs::remove_file(side)?;
        }
    }
    fs::rename(&staged, &live)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::universes;
    use std::io::Write;

    fn setup() -> (PathBuf, PathBuf, Connection) {
        let base = std::env::temp_dir().join(format!("uwb-test-{}", db::new_id()));
        fs::create_dir_all(&base).unwrap();
        let (_info, opened) = projects::create(&base, "P").unwrap();
        (base, opened.dir, opened.conn)
    }

    #[test]
    fn criar_verificar_e_detectar_adulteracao() {
        let (base, dir, conn) = setup();
        universes::create(&conn, "A", None).unwrap();

        let b = create(&conn, &dir, "manual", Some("primeiro")).unwrap();
        assert_eq!(b.manifest.counts.universes, 1);
        assert_eq!(b.manifest.label.as_deref(), Some("primeiro"));
        assert_eq!(list(&dir).unwrap().len(), 1);
        assert!(verify(&dir, &b.id).unwrap().ok);

        // Altera um byte do backup: a verificação precisa falhar.
        let path = dir.join("backups").join(&b.id).join("project.db");
        let mut f = fs::OpenOptions::new().append(true).open(path).unwrap();
        f.write_all(b"x").unwrap();
        drop(f);
        assert!(!verify(&dir, &b.id).unwrap().ok);

        drop(conn);
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn restaurar_volta_ao_estado_do_backup() {
        let (base, dir, conn) = setup();
        universes::create(&conn, "A", None).unwrap();
        let b = create(&conn, &dir, "manual", None).unwrap();
        universes::create(&conn, "B", None).unwrap();
        assert_eq!(universes::list(&conn).unwrap().len(), 2);

        let backup_db = prepare_restore(&dir, &b.id).unwrap();
        drop(conn); // o banco precisa estar fechado
        apply_restore(&dir, &backup_db).unwrap();

        let reopened = db::open(&dir.join("project.db")).unwrap();
        assert_eq!(universes::list(&reopened).unwrap().len(), 1);

        // Id malicioso é recusado.
        assert!(prepare_restore(&dir, "../fora").is_err());

        drop(reopened);
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn automatico_respeita_intervalo_e_retencao() {
        let (base, dir, conn) = setup();

        assert!(auto_if_due(&conn, &dir).unwrap().is_some());
        // Acabou de criar: ainda não está na hora.
        assert!(auto_if_due(&conn, &dir).unwrap().is_none());

        for _ in 0..2 {
            create(&conn, &dir, "auto", None).unwrap();
        }
        assert_eq!(list(&dir).unwrap().len(), 3);

        prune_auto(&dir, 1).unwrap();
        assert_eq!(list(&dir).unwrap().len(), 1);

        drop(conn);
        fs::remove_dir_all(&base).ok();
    }
}