//! Persistent app state in SQLite. Pure storage: no git, no filesystem, no policy.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};

/// Applied in order; a database's `user_version` is how many it has had. Never edit a shipped
/// migration — add a new one.
const MIGRATIONS: &[&str] = &[include_str!("../migrations/0001_init.sql")];

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("cannot create the data directory: {0}")]
    Io(#[from] std::io::Error),
    #[error("this database was written by a newer version of Switchyard (schema {found}, this build knows {known})")]
    TooNew { found: usize, known: usize },
}

pub type StoreResult<T> = Result<T, StoreError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub root_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRow {
    pub id: String,
    pub project_id: String,
    pub kind: String,
    pub name: String,
    pub path: String,
    pub branch: Option<String>,
}

pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    pub fn open(path: &Path) -> StoreResult<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Self::prepare(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn in_memory() -> Self {
        Self::prepare(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn prepare(mut conn: Connection) -> StoreResult<Self> {
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;

        let applied: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        let applied = applied as usize;
        if applied > MIGRATIONS.len() {
            return Err(StoreError::TooNew {
                found: applied,
                known: MIGRATIONS.len(),
            });
        }
        for (index, sql) in MIGRATIONS.iter().enumerate().skip(applied) {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(
                None,
                "user_version",
                u32::try_from(index + 1).unwrap_or(u32::MAX),
            )?;
            tx.commit()?;
        }
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Insert a project together with its `local` workspace, at the end of the list.
    pub fn add_project(&self, name: &str, root_path: &str) -> StoreResult<ProjectRow> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let project = ProjectRow {
            id: new_id(),
            name: name.to_owned(),
            root_path: root_path.to_owned(),
        };
        let next: i64 = tx.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM projects",
            [],
            |row| row.get(0),
        )?;
        tx.execute(
            "INSERT INTO projects (id, name, root_path, sort_order, created_at) VALUES (?, ?, ?, ?, ?)",
            params![project.id, project.name, project.root_path, next, now_ms()],
        )?;
        tx.execute(
            "INSERT INTO workspaces (id, project_id, kind, name, path, created_at)
             VALUES (?, ?, 'local', 'local', ?, ?)",
            params![new_id(), project.id, project.root_path, now_ms()],
        )?;
        tx.commit()?;
        Ok(project)
    }

    pub fn projects(&self) -> StoreResult<Vec<ProjectRow>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT id, name, root_path FROM projects ORDER BY sort_order, created_at")?;
        let rows = stmt.query_map([], |row| {
            Ok(ProjectRow {
                id: row.get(0)?,
                name: row.get(1)?,
                root_path: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn project_by_root(&self, root_path: &str) -> StoreResult<Option<ProjectRow>> {
        Ok(self
            .projects()?
            .into_iter()
            .find(|project| project.root_path == root_path))
    }

    /// Active workspaces of every project; `local` first within each.
    pub fn workspaces(&self) -> StoreResult<Vec<WorkspaceRow>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, kind, name, path, branch FROM workspaces
             WHERE status = 'active'
             ORDER BY project_id, kind = 'local' DESC, sort_order, created_at",
        )?;
        let rows = stmt.query_map([], workspace_from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn workspace(&self, id: &str) -> StoreResult<Option<WorkspaceRow>> {
        Ok(self
            .conn()
            .query_row(
                "SELECT id, project_id, kind, name, path, branch FROM workspaces WHERE id = ?",
                [id],
                workspace_from_row,
            )
            .optional()?)
    }

    /// Forget a project and its workspaces. Returns whether it existed.
    pub fn remove_project(&self, id: &str) -> StoreResult<bool> {
        Ok(self
            .conn()
            .execute("DELETE FROM projects WHERE id = ?", [id])?
            > 0)
    }

    /// Put projects in the given order. Ids not mentioned keep their relative order, after.
    pub fn reorder_projects(&self, ordered_ids: &[String]) -> StoreResult<()> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let count = i64::try_from(ordered_ids.len()).unwrap_or(i64::MAX);
        tx.execute("UPDATE projects SET sort_order = sort_order + ?", [count])?;
        for (index, id) in ordered_ids.iter().enumerate() {
            tx.execute(
                "UPDATE projects SET sort_order = ? WHERE id = ?",
                params![i64::try_from(index).unwrap_or(i64::MAX), id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn ui_state(&self) -> StoreResult<BTreeMap<String, String>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT key, value FROM ui_state")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn set_ui_state(&self, key: &str, value: &str) -> StoreResult<()> {
        self.conn().execute(
            "INSERT INTO ui_state (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        )?;
        Ok(())
    }

    fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn workspace_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkspaceRow> {
    Ok(WorkspaceRow {
        id: row.get(0)?,
        project_id: row.get(1)?,
        kind: row.get(2)?,
        name: row.get(3)?,
        path: row.get(4)?,
        branch: row.get(5)?,
    })
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(store: &Store) -> Vec<String> {
        store
            .projects()
            .unwrap()
            .into_iter()
            .map(|p| p.name)
            .collect()
    }

    #[test]
    fn a_project_arrives_with_its_local_workspace() {
        let store = Store::in_memory();
        let project = store.add_project("app", "/code/app").unwrap();

        let workspaces = store.workspaces().unwrap();
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].project_id, project.id);
        assert_eq!(workspaces[0].kind, "local");
        assert_eq!(workspaces[0].path, "/code/app");
        assert_eq!(
            store.workspace(&workspaces[0].id).unwrap(),
            Some(workspaces[0].clone())
        );
    }

    #[test]
    fn the_same_root_cannot_be_added_twice() {
        let store = Store::in_memory();
        store.add_project("app", "/code/app").unwrap();
        assert!(store.add_project("again", "/code/app").is_err());
        assert_eq!(
            store.project_by_root("/code/app").unwrap().unwrap().name,
            "app"
        );
        assert_eq!(store.project_by_root("/code/other").unwrap(), None);
    }

    #[test]
    fn projects_keep_insertion_order_until_reordered() {
        let store = Store::in_memory();
        let ids: Vec<_> = ["a", "b", "c"]
            .iter()
            .map(|n| store.add_project(n, &format!("/{n}")).unwrap().id)
            .collect();
        assert_eq!(names(&store), ["a", "b", "c"]);

        store
            .reorder_projects(&[ids[2].clone(), ids[0].clone()])
            .unwrap();
        assert_eq!(
            names(&store),
            ["c", "a", "b"],
            "unmentioned projects go last"
        );

        store.add_project("d", "/d").unwrap();
        assert_eq!(names(&store), ["c", "a", "b", "d"]);
    }

    #[test]
    fn removing_a_project_takes_its_workspaces_along() {
        let store = Store::in_memory();
        let project = store.add_project("app", "/code/app").unwrap();
        assert!(store.remove_project(&project.id).unwrap());
        assert!(store.workspaces().unwrap().is_empty());
        assert!(!store.remove_project(&project.id).unwrap());
    }

    #[test]
    fn ui_state_upserts() {
        let store = Store::in_memory();
        store.set_ui_state("selected", "a").unwrap();
        store.set_ui_state("selected", "b").unwrap();
        store.set_ui_state("expanded", "[]").unwrap();
        let state = store.ui_state().unwrap();
        assert_eq!(state["selected"], "b");
        assert_eq!(state.len(), 2);
    }

    #[test]
    fn data_survives_reopening_and_migrations_run_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("switchyard.db");
        Store::open(&path)
            .unwrap()
            .add_project("app", "/code/app")
            .unwrap();
        let reopened = Store::open(&path).unwrap();
        assert_eq!(names(&reopened), ["app"]);
    }

    #[test]
    fn a_database_from_the_future_is_refused_not_mangled() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("switchyard.db");
        drop(Store::open(&path).unwrap());
        Connection::open(&path)
            .unwrap()
            .pragma_update(None, "user_version", 999)
            .unwrap();
        assert!(matches!(
            Store::open(&path),
            Err(StoreError::TooNew { found: 999, .. })
        ));
    }
}
