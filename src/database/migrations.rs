use rusqlite::Connection;

use crate::errors::{OmniDevError, Result};

const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("../../migrations/0001_initial.sql"))];

pub fn latest_version() -> i64 {
    MIGRATIONS.last().map_or(0, |(version, _)| *version)
}

pub fn current_version(connection: &Connection) -> Result<i64> {
    Ok(connection.query_row("PRAGMA user_version", [], |row| row.get(0))?)
}

pub fn apply(connection: &mut Connection) -> Result<()> {
    let current = current_version(connection)?;
    let latest = latest_version();
    if current > latest {
        return Err(OmniDevError::SchemaTooNew {
            found: current,
            supported: latest,
        });
    }
    for (version, sql) in MIGRATIONS.iter().filter(|(version, _)| *version > current) {
        let transaction = connection.transaction()?;
        transaction.execute_batch(sql)?;
        transaction.pragma_update(None, "user_version", *version)?;
        transaction.commit()?;
    }
    Ok(())
}
