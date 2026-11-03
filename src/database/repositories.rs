use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::errors::{OmniDevError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryRecord {
    pub id: i64,
    pub name: String,
    pub path: PathBuf,
    pub is_default: bool,
    pub head_commit: Option<String>,
    pub added_at: String,
    pub last_synced_at: Option<String>,
}

const COLUMNS: &str = "id, name, path, is_default, head_commit, added_at, last_synced_at";

fn map_row(row: &Row<'_>) -> rusqlite::Result<RepositoryRecord> {
    let path: String = row.get(2)?;
    Ok(RepositoryRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        path: PathBuf::from(path),
        is_default: row.get::<_, i64>(3)? == 1,
        head_commit: row.get(4)?,
        added_at: row.get(5)?,
        last_synced_at: row.get(6)?,
    })
}

pub fn insert(connection: &Connection, name: &str, path: &Path, added_at: &str) -> Result<i64> {
    let path_text = path.to_string_lossy().into_owned();
    let existing: Option<i64> = connection
        .query_row(
            "SELECT id FROM repositories WHERE path = ?1",
            params![path_text],
            |row| row.get(0),
        )
        .optional()?;
    if existing.is_some() {
        return Err(OmniDevError::InvalidInput(format!(
            "{} is already tracked by OmniDev",
            path.display()
        )));
    }
    let has_default: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM repositories WHERE is_default = 1)",
        [],
        |row| row.get(0),
    )?;
    connection.execute(
        "INSERT INTO repositories (name, path, is_default, added_at) VALUES (?1, ?2, ?3, ?4)",
        params![name, path_text, i64::from(!has_default), added_at],
    )?;
    Ok(connection.last_insert_rowid())
}

pub fn list(connection: &Connection) -> Result<Vec<RepositoryRecord>> {
    let mut statement = connection.prepare(&format!(
        "SELECT {COLUMNS} FROM repositories ORDER BY name, id"
    ))?;
    let rows = statement.query_map([], map_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn find_by_id(connection: &Connection, id: i64) -> Result<Option<RepositoryRecord>> {
    let record = connection
        .query_row(
            &format!("SELECT {COLUMNS} FROM repositories WHERE id = ?1"),
            params![id],
            map_row,
        )
        .optional()?;
    Ok(record)
}

pub fn find(connection: &Connection, selector: &str) -> Result<RepositoryRecord> {
    let mut matches = list(connection)?
        .into_iter()
        .filter(|record| record.path.to_string_lossy() == selector || record.name == selector)
        .collect::<Vec<_>>();
    match matches.len() {
        0 => Err(OmniDevError::RepositoryNotFound(selector.to_owned())),
        1 => Ok(matches.remove(0)),
        _ => {
            let by_path = matches
                .iter()
                .position(|record| record.path.to_string_lossy() == selector);
            match by_path {
                Some(index) => Ok(matches.remove(index)),
                None => Err(OmniDevError::AmbiguousRepository(selector.to_owned())),
            }
        }
    }
}

pub fn find_by_path(connection: &Connection, path: &Path) -> Result<Option<RepositoryRecord>> {
    let record = connection
        .query_row(
            &format!("SELECT {COLUMNS} FROM repositories WHERE path = ?1"),
            params![path.to_string_lossy()],
            map_row,
        )
        .optional()?;
    Ok(record)
}

pub fn default_repository(connection: &Connection) -> Result<Option<RepositoryRecord>> {
    let record = connection
        .query_row(
            &format!("SELECT {COLUMNS} FROM repositories WHERE is_default = 1"),
            [],
            map_row,
        )
        .optional()?;
    Ok(record)
}

pub fn set_default(connection: &mut Connection, id: i64) -> Result<()> {
    let transaction = connection.transaction()?;
    transaction.execute(
        "UPDATE repositories SET is_default = 0 WHERE is_default = 1",
        [],
    )?;
    transaction.execute(
        "UPDATE repositories SET is_default = 1 WHERE id = ?1",
        params![id],
    )?;
    transaction.commit()?;
    Ok(())
}

pub fn remove(connection: &Connection, id: i64) -> Result<()> {
    connection.execute("DELETE FROM repositories WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn record_sync(
    connection: &Connection,
    id: i64,
    head_commit: Option<&str>,
    synced_at: &str,
) -> Result<()> {
    connection.execute(
        "UPDATE repositories SET head_commit = ?2, last_synced_at = ?3 WHERE id = ?1",
        params![id, head_commit, synced_at],
    )?;
    Ok(())
}
