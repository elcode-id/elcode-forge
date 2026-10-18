use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// File-level churn hotspot summary computed from Git commit history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotspotEntry {
    pub path: String,
    pub commit_count: u32,
    pub contributors_count: usize,
}

/// Parses `git log --format="COMMIT:%ae" --name-only` style output into top `top_n` churn hotspots
/// ordered by `commit_count` descending then `path` ascending.
pub fn compute_hotspots_from_log(git_log_text: &str, top_n: usize) -> Vec<HotspotEntry> {
    let mut commits_per_file: BTreeMap<String, u32> = BTreeMap::new();
    let mut authors_per_file: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut current_author = String::new();

    for raw_line in git_log_text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(author) = line.strip_prefix("COMMIT:") {
            current_author = author.trim().to_string();
            continue;
        }

        let entry = commits_per_file.entry(line.to_string()).or_insert(0);
        *entry += 1;

        if !current_author.is_empty() {
            authors_per_file
                .entry(line.to_string())
                .or_default()
                .insert(current_author.clone());
        }
    }

    let mut hotspots: Vec<HotspotEntry> = commits_per_file
        .into_iter()
        .map(|(path, commit_count)| {
            let contributors_count = authors_per_file.get(&path).map(BTreeSet::len).unwrap_or(1);
            HotspotEntry {
                path,
                commit_count,
                contributors_count,
            }
        })
        .collect();

    hotspots.sort_by(|a, b| {
        b.commit_count
            .cmp(&a.commit_count)
            .then_with(|| a.path.cmp(&b.path))
    });
    hotspots.truncate(top_n);
    hotspots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_hotspots_by_commit_frequency_and_contributor_count() {
        let log = r#"
COMMIT:alice@elcode.id
src/services/userService.ts
src/db/client.ts

COMMIT:bob@elcode.id
src/services/userService.ts

COMMIT:charlie@elcode.id
src/services/userService.ts
src/pages/index.tsx
"#;

        let hotspots = compute_hotspots_from_log(log, 2);
        assert_eq!(hotspots.len(), 2);
        assert_eq!(
            hotspots[0],
            HotspotEntry {
                path: "src/services/userService.ts".to_string(),
                commit_count: 3,
                contributors_count: 3,
            }
        );
        assert_eq!(
            hotspots[1],
            HotspotEntry {
                path: "src/db/client.ts".to_string(),
                commit_count: 1,
                contributors_count: 1,
            }
        );
    }
}
