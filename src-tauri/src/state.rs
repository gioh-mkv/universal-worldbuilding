use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;

use crate::db;
use crate::error::{AppError, Result};

pub struct OpenProject {
    pub conn: Connection,
    pub dir: PathBuf,
}

#[derive(Default)]
pub struct AppState {
    project: Mutex<Option<OpenProject>>,
}

fn poisoned() -> AppError {
    AppError::msg("estado interno corrompido")
}

fn none_open() -> AppError {
    AppError::msg("nenhum projeto aberto")
}

impl AppState {
    pub fn set(&self, p: OpenProject) -> Result<()> {
        let mut g = self.project.lock().map_err(|_| poisoned())?;
        *g = Some(p);
        Ok(())
    }

    /// Fecha o projeto aberto (a conexão é encerrada).
    pub fn close(&self) -> Result<()> {
        let mut g = self.project.lock().map_err(|_| poisoned())?;
        *g = None;
        Ok(())
    }

    pub fn current_dir(&self) -> Result<Option<PathBuf>> {
        let g = self.project.lock().map_err(|_| poisoned())?;
        Ok(g.as_ref().map(|p| p.dir.clone()))
    }

    /// Executa `f` com a conexão do projeto aberto.
    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let g = self.project.lock().map_err(|_| poisoned())?;
        let p = g.as_ref().ok_or_else(none_open)?;
        f(&p.conn)
    }

    /// Como `with`, mas também entrega a pasta do projeto.
    pub fn with_project<T>(&self, f: impl FnOnce(&Connection, &Path) -> Result<T>) -> Result<T> {
        let g = self.project.lock().map_err(|_| poisoned())?;
        let p = g.as_ref().ok_or_else(none_open)?;
        f(&p.conn, &p.dir)
    }

    /// Entrega o projeto aberto a `f` (que pode fechá-lo e mexer nos arquivos)
    /// e SEMPRE reabre o banco depois, tenha `f` dado certo ou não.
    pub fn swap<T>(&self, f: impl FnOnce(OpenProject) -> Result<T>) -> Result<T> {
        let mut g = self.project.lock().map_err(|_| poisoned())?;
        let open = g.take().ok_or_else(none_open)?;
        let dir = open.dir.clone();
        let result = f(open); // `open` é consumido aqui; a conexão é fechada ao ser descartada
        match db::open(&dir.join("project.db")) {
            Ok(conn) => {
                *g = Some(OpenProject { conn, dir });
                result
            }
            Err(e) => Err(e),
        }
    }
}