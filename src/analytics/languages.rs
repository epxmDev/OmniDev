use std::collections::BTreeMap;

use git2::{ObjectType, Oid, Repository, TreeWalkMode, TreeWalkResult};

use crate::database::languages::LanguageTally;
use crate::errors::Result;
use crate::git::repository::head_hash;

const EXTENSION_LANGUAGES: &[(&str, &str)] = &[
    ("rs", "Rust"),
    ("py", "Python"),
    ("js", "JavaScript"),
    ("mjs", "JavaScript"),
    ("cjs", "JavaScript"),
    ("jsx", "JavaScript"),
    ("ts", "TypeScript"),
    ("tsx", "TypeScript"),
    ("go", "Go"),
    ("c", "C"),
    ("h", "C"),
    ("cc", "C++"),
    ("cpp", "C++"),
    ("cxx", "C++"),
    ("hpp", "C++"),
    ("hh", "C++"),
    ("java", "Java"),
    ("kt", "Kotlin"),
    ("kts", "Kotlin"),
    ("swift", "Swift"),
    ("rb", "Ruby"),
    ("php", "PHP"),
    ("cs", "C#"),
    ("sh", "Shell"),
    ("bash", "Shell"),
    ("zsh", "Shell"),
    ("html", "HTML"),
    ("css", "CSS"),
    ("scss", "SCSS"),
    ("sql", "SQL"),
    ("lua", "Lua"),
    ("dart", "Dart"),
    ("scala", "Scala"),
    ("hs", "Haskell"),
    ("ex", "Elixir"),
    ("exs", "Elixir"),
    ("zig", "Zig"),
    ("vue", "Vue"),
    ("svelte", "Svelte"),
    ("r", "R"),
    ("md", "Markdown"),
    ("toml", "TOML"),
    ("yaml", "YAML"),
    ("yml", "YAML"),
    ("json", "JSON"),
];

const EXCLUDED_DIRECTORIES: &[&str] = &[
    "vendor",
    "node_modules",
    "target",
    "build",
    "dist",
    "third_party",
    "__pycache__",
];

const GENERATED_SUFFIXES: &[&str] = &[
    ".min.js",
    ".min.css",
    ".pb.go",
    "_pb2.py",
    ".generated.ts",
    ".g.dart",
];

const GENERATED_FILE_NAMES: &[&str] = &[
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "Cargo.lock",
    "composer.lock",
    "poetry.lock",
];

pub fn language_for_path(path: &str) -> Option<&'static str> {
    let file_name = path.rsplit('/').next()?;
    let (stem, extension) = file_name.rsplit_once('.')?;
    if stem.is_empty() {
        return None;
    }
    let extension = extension.to_ascii_lowercase();
    EXTENSION_LANGUAGES
        .iter()
        .find(|(known, _)| *known == extension.as_str())
        .map(|(_, language)| *language)
}

pub fn is_excluded(path: &str, exclude_generated: bool) -> bool {
    let mut segments = path.split('/').peekable();
    let mut file_name = "";
    while let Some(segment) = segments.next() {
        if segments.peek().is_none() {
            file_name = segment;
        } else if EXCLUDED_DIRECTORIES.contains(&segment) || segment.starts_with('.') {
            return true;
        }
    }
    if !exclude_generated {
        return false;
    }
    GENERATED_FILE_NAMES.contains(&file_name)
        || GENERATED_SUFFIXES
            .iter()
            .any(|suffix| file_name.ends_with(suffix))
}

pub fn count_lines(content: &[u8]) -> u64 {
    let newlines = content.iter().filter(|byte| **byte == b'\n').count() as u64;
    if content.is_empty() || content.last() == Some(&b'\n') {
        newlines
    } else {
        newlines + 1
    }
}

pub fn tally_repository(
    repository: &Repository,
    exclude_generated: bool,
) -> Result<Vec<LanguageTally>> {
    let Some(hash) = head_hash(repository)? else {
        return Ok(Vec::new());
    };
    let commit = repository.find_commit(Oid::from_str(&hash)?)?;
    let tree = commit.tree()?;
    let mut blobs: Vec<(String, Oid)> = Vec::new();
    tree.walk(TreeWalkMode::PreOrder, |root, entry| {
        if entry.kind() == Some(ObjectType::Blob) {
            let name = String::from_utf8_lossy(entry.name_bytes());
            blobs.push((format!("{root}{name}"), entry.id()));
        }
        TreeWalkResult::Ok
    })?;
    let mut totals: BTreeMap<&'static str, (u64, u64)> = BTreeMap::new();
    for (path, oid) in blobs {
        if is_excluded(&path, exclude_generated) {
            continue;
        }
        let Some(language) = language_for_path(&path) else {
            continue;
        };
        let blob = repository.find_blob(oid)?;
        if blob.is_binary() {
            continue;
        }
        let entry = totals.entry(language).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += count_lines(blob.content());
    }
    let mut tallies: Vec<LanguageTally> = totals
        .into_iter()
        .map(|(language, (files, lines))| LanguageTally {
            language: language.to_owned(),
            files,
            lines,
        })
        .collect();
    tallies.sort_by(|left, right| {
        right
            .lines
            .cmp(&left.lines)
            .then(left.language.cmp(&right.language))
    });
    Ok(tallies)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_extensions_case_insensitively() {
        assert_eq!(language_for_path("src/main.rs"), Some("Rust"));
        assert_eq!(language_for_path("app/View.TSX"), Some("TypeScript"));
        assert_eq!(language_for_path("notes/README"), None);
        assert_eq!(language_for_path(".gitignore"), None);
    }

    #[test]
    fn excludes_vendored_and_generated_paths() {
        assert!(is_excluded("vendor/lib/a.go", false));
        assert!(is_excluded("web/node_modules/x/index.js", false));
        assert!(is_excluded(".github/workflows/ci.yml", false));
        assert!(is_excluded("web/bundle.min.js", true));
        assert!(is_excluded("package-lock.json", true));
        assert!(!is_excluded("package-lock.json", false));
        assert!(!is_excluded("src/lib.rs", true));
    }

    #[test]
    fn counts_lines_without_trailing_newline() {
        assert_eq!(count_lines(b""), 0);
        assert_eq!(count_lines(b"one\n"), 1);
        assert_eq!(count_lines(b"one\ntwo"), 2);
        assert_eq!(count_lines(b"a\nb\nc\n"), 3);
    }
}
