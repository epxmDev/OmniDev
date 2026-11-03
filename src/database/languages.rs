use rusqlite::{Connection, params};

use crate::errors::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageTally {
    pub language: String,
    pub files: u64,
    pub lines: u64,
}

pub fn replace_repository_languages(
    connection: &Connection,
    repository_id: i64,
    tallies: &[LanguageTally],
) -> Result<()> {
    connection.execute(
        "DELETE FROM language_statistics WHERE repository_id = ?1",
        params![repository_id],
    )?;
    let mut statement = connection.prepare(
        "INSERT INTO language_statistics (repository_id, language, files, lines) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for tally in tallies {
        statement.execute(params![
            repository_id,
            tally.language,
            tally.files as i64,
            tally.lines as i64
        ])?;
    }
    Ok(())
}

pub fn aggregate_languages(
    connection: &Connection,
    repository_id: Option<i64>,
) -> Result<Vec<LanguageTally>> {
    let mut statement = connection.prepare(
        "SELECT language, SUM(files), SUM(lines) FROM language_statistics
         WHERE (?1 IS NULL OR repository_id = ?1)
         GROUP BY language
         ORDER BY SUM(lines) DESC, language",
    )?;
    let rows = statement.query_map(params![repository_id], |row| {
        Ok(LanguageTally {
            language: row.get(0)?,
            files: row.get::<_, i64>(1)?.max(0) as u64,
            lines: row.get::<_, i64>(2)?.max(0) as u64,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}
