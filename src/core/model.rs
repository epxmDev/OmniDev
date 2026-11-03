use chrono::NaiveDate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInfo {
    pub hash: String,
    pub author_name: String,
    pub author_email: String,
    pub committed_at: i64,
    pub utc_offset_minutes: i32,
    pub summary: String,
    pub is_merge: bool,
    pub insertions: u64,
    pub deletions: u64,
    pub files_changed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityRecord {
    pub repository_id: i64,
    pub repository_name: String,
    pub day: NaiveDate,
    pub author_email: String,
    pub insertions: u64,
    pub deletions: u64,
    pub is_merge: bool,
}
