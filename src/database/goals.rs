use rusqlite::{Connection, OptionalExtension, params};

use crate::errors::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalRecord {
    pub kind: String,
    pub target: u64,
    pub created_at: String,
}

pub fn upsert_goal(
    connection: &Connection,
    kind: &str,
    target: u64,
    created_at: &str,
) -> Result<()> {
    connection.execute(
        "INSERT INTO goals (kind, target, created_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(kind) DO UPDATE SET target = excluded.target, created_at = excluded.created_at",
        params![kind, target as i64, created_at],
    )?;
    Ok(())
}

pub fn delete_goal(connection: &Connection, kind: &str) -> Result<bool> {
    Ok(connection.execute("DELETE FROM goals WHERE kind = ?1", params![kind])? == 1)
}

pub fn list_goals(connection: &Connection) -> Result<Vec<GoalRecord>> {
    let mut statement =
        connection.prepare("SELECT kind, target, created_at FROM goals ORDER BY kind")?;
    let rows = statement.query_map([], |row| {
        Ok(GoalRecord {
            kind: row.get(0)?,
            target: row.get::<_, i64>(1)?.max(0) as u64,
            created_at: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn unlocked_achievements(connection: &Connection) -> Result<Vec<(String, String)>> {
    let mut statement =
        connection.prepare("SELECT code, unlocked_at FROM achievements ORDER BY unlocked_at")?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn unlock_achievement(connection: &Connection, code: &str, unlocked_at: &str) -> Result<bool> {
    Ok(connection.execute(
        "INSERT OR IGNORE INTO achievements (code, unlocked_at) VALUES (?1, ?2)",
        params![code, unlocked_at],
    )? == 1)
}

pub fn get_metadata(connection: &Connection, key: &str) -> Result<Option<String>> {
    Ok(connection
        .query_row(
            "SELECT value FROM application_metadata WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn set_metadata(connection: &Connection, key: &str, value: &str) -> Result<()> {
    connection.execute(
        "INSERT INTO application_metadata (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}
