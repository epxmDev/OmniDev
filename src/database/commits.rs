use std::collections::HashSet;

use chrono::NaiveDate;
use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};

use crate::core::activity::{DateRange, format_day};
use crate::core::model::{ActivityRecord, CommitInfo};
use crate::errors::Result;

pub fn upsert_contributor(connection: &Connection, email: &str, name: &str) -> Result<i64> {
    connection.execute(
        "INSERT INTO contributors (email, name) VALUES (?1, ?2)
         ON CONFLICT(email) DO UPDATE SET name = excluded.name",
        params![email, name],
    )?;
    let id = connection.query_row(
        "SELECT id FROM contributors WHERE email = ?1",
        params![email],
        |row| row.get(0),
    )?;
    Ok(id)
}

pub fn known_hashes(connection: &Connection, repository_id: i64) -> Result<HashSet<String>> {
    let mut statement = connection.prepare("SELECT hash FROM commits WHERE repository_id = ?1")?;
    let rows = statement.query_map(params![repository_id], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<HashSet<_>>>()?)
}

pub fn insert_commit(
    connection: &Connection,
    repository_id: i64,
    commit: &CommitInfo,
    day: NaiveDate,
) -> Result<()> {
    let contributor_id = upsert_contributor(connection, &commit.author_email, &commit.author_name)?;
    connection.execute(
        "INSERT OR IGNORE INTO commits (
            repository_id, hash, contributor_id, author_name, committed_at, utc_offset_minutes,
            local_day, summary, is_merge, insertions, deletions, files_changed
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            repository_id,
            commit.hash,
            contributor_id,
            commit.author_name,
            commit.committed_at,
            commit.utc_offset_minutes,
            format_day(day),
            commit.summary,
            i64::from(commit.is_merge),
            commit.insertions as i64,
            commit.deletions as i64,
            commit.files_changed as i64,
        ],
    )?;
    Ok(())
}

pub fn clear_repository_commits(connection: &Connection, repository_id: i64) -> Result<()> {
    connection.execute(
        "DELETE FROM commits WHERE repository_id = ?1",
        params![repository_id],
    )?;
    connection.execute(
        "DELETE FROM daily_activity WHERE repository_id = ?1",
        params![repository_id],
    )?;
    Ok(())
}

pub fn rebuild_daily_activity(connection: &Connection, repository_id: i64) -> Result<()> {
    connection.execute(
        "DELETE FROM daily_activity WHERE repository_id = ?1",
        params![repository_id],
    )?;
    connection.execute(
        "INSERT INTO daily_activity (repository_id, day, commits, merge_commits, insertions, deletions)
         SELECT repository_id, local_day, COUNT(*), SUM(is_merge), SUM(insertions), SUM(deletions)
         FROM commits WHERE repository_id = ?1 GROUP BY local_day",
        params![repository_id],
    )?;
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct CommitFilter {
    pub repository_id: Option<i64>,
    pub author: Option<String>,
    pub range: Option<DateRange>,
    pub search: Option<String>,
    pub include_merges: bool,
}

fn escape_like(term: &str) -> String {
    let mut escaped = String::with_capacity(term.len());
    for character in term.chars() {
        if matches!(character, '%' | '_' | '\\') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    format!("%{escaped}%")
}

fn filter_clauses(filter: &CommitFilter, alias: &str) -> (String, Vec<Value>) {
    let mut clauses = Vec::new();
    let mut values = Vec::new();
    if let Some(repository_id) = filter.repository_id {
        clauses.push(format!("{alias}.repository_id = ?"));
        values.push(Value::Integer(repository_id));
    }
    if let Some(range) = filter.range {
        clauses.push(format!("{alias}.local_day BETWEEN ? AND ?"));
        values.push(Value::Text(format_day(range.start)));
        values.push(Value::Text(format_day(range.end)));
    }
    if !filter.include_merges {
        clauses.push(format!("{alias}.is_merge = 0"));
    }
    if let Some(author) = &filter.author {
        let pattern = escape_like(author);
        clauses.push(format!(
            "(k.name LIKE ? ESCAPE '\\' OR k.email LIKE ? ESCAPE '\\' OR {alias}.author_name LIKE ? ESCAPE '\\')"
        ));
        values.push(Value::Text(pattern.clone()));
        values.push(Value::Text(pattern.clone()));
        values.push(Value::Text(pattern));
    }
    if let Some(search) = &filter.search {
        clauses.push(format!("{alias}.summary LIKE ? ESCAPE '\\'"));
        values.push(Value::Text(escape_like(search)));
    }
    let condition = if clauses.is_empty() {
        "1 = 1".to_owned()
    } else {
        clauses.join(" AND ")
    };
    (condition, values)
}

pub fn load_activity_records(
    connection: &Connection,
    filter: &CommitFilter,
) -> Result<Vec<ActivityRecord>> {
    let (condition, values) = filter_clauses(filter, "c");
    let sql = format!(
        "SELECT c.repository_id, r.name, c.local_day, k.email, c.insertions, c.deletions, c.is_merge
         FROM commits c
         JOIN repositories r ON r.id = c.repository_id
         JOIN contributors k ON k.id = c.contributor_id
         WHERE {condition}"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(values.iter()), |row| {
        let day: String = row.get(2)?;
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            day,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)? == 1,
        ))
    })?;
    let mut records = Vec::new();
    for row in rows {
        let (repository_id, repository_name, day, author_email, insertions, deletions, is_merge) =
            row?;
        records.push(ActivityRecord {
            repository_id,
            repository_name,
            day: parse_day(&day)?,
            author_email,
            insertions: insertions.max(0) as u64,
            deletions: deletions.max(0) as u64,
            is_merge,
        });
    }
    Ok(records)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitListing {
    pub repository_name: String,
    pub repository_path: String,
    pub hash: String,
    pub author_name: String,
    pub author_email: String,
    pub summary: String,
    pub day: NaiveDate,
    pub insertions: u64,
    pub deletions: u64,
    pub files_changed: u64,
    pub is_merge: bool,
}

pub fn count_commits(connection: &Connection, filter: &CommitFilter) -> Result<u64> {
    let (condition, values) = filter_clauses(filter, "c");
    let sql = format!(
        "SELECT COUNT(*) FROM commits c
         JOIN contributors k ON k.id = c.contributor_id
         WHERE {condition}"
    );
    let count: i64 =
        connection.query_row(&sql, params_from_iter(values.iter()), |row| row.get(0))?;
    Ok(count.max(0) as u64)
}

fn parse_day(text: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").map_err(|error| {
        crate::errors::OmniDevError::InvalidInput(format!("stored day '{text}': {error}"))
    })
}

pub fn list_commits(
    connection: &Connection,
    filter: &CommitFilter,
    limit: u64,
    offset: u64,
) -> Result<Vec<CommitListing>> {
    let (condition, mut values) = filter_clauses(filter, "c");
    values.push(Value::Integer(limit as i64));
    values.push(Value::Integer(offset as i64));
    let sql = format!(
        "SELECT r.name, c.hash, c.author_name, k.email, c.summary, c.local_day, r.path,
                c.insertions, c.deletions, c.files_changed, c.is_merge
         FROM commits c
         JOIN repositories r ON r.id = c.repository_id
         JOIN contributors k ON k.id = c.contributor_id
         WHERE {condition}
         ORDER BY c.committed_at DESC, c.id DESC
         LIMIT ? OFFSET ?"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(values.iter()), |row| {
        Ok((
            row.get::<_, String>(5)?,
            CommitListing {
                repository_name: row.get(0)?,
                repository_path: row.get(6)?,
                hash: row.get(1)?,
                author_name: row.get(2)?,
                author_email: row.get(3)?,
                summary: row.get(4)?,
                day: NaiveDate::MIN,
                insertions: row.get::<_, i64>(7)?.max(0) as u64,
                deletions: row.get::<_, i64>(8)?.max(0) as u64,
                files_changed: row.get::<_, i64>(9)?.max(0) as u64,
                is_merge: row.get::<_, i64>(10)? == 1,
            },
        ))
    })?;
    let mut listings = Vec::new();
    for row in rows {
        let (day_text, mut listing) = row?;
        listing.day = parse_day(&day_text)?;
        listings.push(listing);
    }
    Ok(listings)
}

pub fn daily_totals(
    connection: &Connection,
    repository_id: Option<i64>,
    include_merges: bool,
) -> Result<Vec<(NaiveDate, u64)>> {
    let sql = String::from(
        "SELECT day, SUM(commits) - CASE WHEN ?1 THEN 0 ELSE SUM(merge_commits) END
         FROM daily_activity WHERE (?2 IS NULL OR repository_id = ?2) GROUP BY day ORDER BY day",
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params![include_merges, repository_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    let mut totals = Vec::new();
    for row in rows {
        let (day, count) = row?;
        totals.push((parse_day(&day)?, count.max(0) as u64));
    }
    Ok(totals)
}

pub fn repository_commit_counts(
    connection: &Connection,
    include_merges: bool,
) -> Result<Vec<(i64, String, u64)>> {
    let mut statement = connection.prepare(
        "SELECT r.id, r.name, SUM(d.commits) - CASE WHEN ?1 THEN 0 ELSE SUM(d.merge_commits) END
         FROM repositories r
         LEFT JOIN daily_activity d ON d.repository_id = r.id
         GROUP BY r.id, r.name
         ORDER BY 3 DESC, r.name",
    )?;
    let rows = statement.query_map(params![include_merges], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<i64>>(2)?,
        ))
    })?;
    let mut counts = Vec::new();
    for row in rows {
        let (id, name, count) = row?;
        counts.push((id, name, count.unwrap_or(0).max(0) as u64));
    }
    Ok(counts)
}

pub fn commit_exists(connection: &Connection, repository_id: i64, hash: &str) -> Result<bool> {
    let found: Option<i64> = connection
        .query_row(
            "SELECT 1 FROM commits WHERE repository_id = ?1 AND hash = ?2",
            params![repository_id, hash],
            |row| row.get(0),
        )
        .optional()?;
    Ok(found.is_some())
}
