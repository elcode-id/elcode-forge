//! Robust, zero-panic version parser and interval comparator for OSV vulnerability matching.

use std::cmp::Ordering;

/// Parsed representation of a software package version across ecosystems.
/// Handles standard SemVer (2.0.0), CalVer, and non-strict release strings without panicking.
#[derive(Debug, Clone)]
pub struct Version {
    /// Ordered numeric segments (e.g. `4.17.15` -> `[4, 17, 15]`).
    pub numeric_parts: Vec<u64>,
    /// Optional pre-release identifier (e.g. `"alpha.1"`).
    pub prerelease: Option<String>,
    /// Original raw version string.
    pub raw: String,
}

impl Version {
    /// Parses an arbitrary version string into a normalized [`Version`].
    /// Automatically strips leading `v` or `V` and separates pre-release tags.
    pub fn parse(raw_str: &str) -> Self {
        let trimmed = raw_str.trim();
        let without_v = trimmed.strip_prefix(['v', 'V']).unwrap_or(trimmed);

        // Strip build metadata after '+' if present
        let without_build = match without_v.split_once('+') {
            Some((left, _)) => left,
            None => without_v,
        };

        // Separate pre-release tag after '-'
        let (numeric_str, prerelease) = match without_build.split_once('-') {
            Some((left, right)) => (left, Some(right.to_string())),
            None => (without_build, None),
        };

        let mut numeric_parts = Vec::new();
        for segment in numeric_str.split('.') {
            // Extract leading digits from segment if any non-digit follows
            let digits: String = segment.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(num) = digits.parse::<u64>() {
                numeric_parts.push(num);
            } else if !digits.is_empty() {
                // If overflowed u64, clamp or fallback
                numeric_parts.push(u64::MAX);
            }
        }

        Self {
            numeric_parts,
            prerelease,
            raw: raw_str.to_string(),
        }
    }

    /// Returns `true` if this version represents "0" (all versions introduced).
    pub fn is_zero(&self) -> bool {
        self.numeric_parts.iter().all(|&n| n == 0) && self.prerelease.is_none()
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Version {}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let max_len = self.numeric_parts.len().max(other.numeric_parts.len());
        for i in 0..max_len {
            let a = self.numeric_parts.get(i).copied().unwrap_or(0);
            let b = other.numeric_parts.get(i).copied().unwrap_or(0);
            match a.cmp(&b) {
                Ordering::Equal => continue,
                diff => return diff,
            }
        }

        // SemVer 2.0.0 Rule 9: Normal version has higher precedence than a pre-release version.
        match (&self.prerelease, &other.prerelease) {
            (None, None) => Ordering::Equal,
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (Some(a), Some(b)) => a.cmp(b),
        }
    }
}

/// Evaluates if `candidate_ver` is affected by an OSV event sequence.
///
/// An event sequence contains alternating `introduced` and `fixed` or `last_affected` points:
/// - `introduced: "0"` or `"x.y.z"`
/// - `fixed: "a.b.c"` (candidate is affected if candidate >= introduced && candidate < fixed)
/// - `last_affected: "a.b.c"` (candidate is affected if candidate >= introduced && candidate <= last_affected)
pub fn is_version_in_range(
    candidate: &Version,
    events: &[(Option<String>, Option<String>, Option<String>)], // (introduced, fixed, last_affected)
) -> bool {
    let mut current_introduced: Option<Version> = None;

    for (intro_opt, fixed_opt, last_affected_opt) in events {
        if let Some(intro_str) = intro_opt {
            current_introduced = Some(Version::parse(intro_str));
        }

        if let Some(fixed_str) = fixed_opt {
            let fixed_ver = Version::parse(fixed_str);
            if let Some(intro_ver) = &current_introduced {
                if candidate >= intro_ver && candidate < &fixed_ver {
                    return true;
                }
            }
            current_introduced = None;
        }

        if let Some(last_str) = last_affected_opt {
            let last_ver = Version::parse(last_str);
            if let Some(intro_ver) = &current_introduced {
                if candidate >= intro_ver && candidate <= &last_ver {
                    return true;
                }
            }
            current_introduced = None;
        }
    }

    // If an introduced point remained open without a fixed bound:
    if let Some(intro_ver) = current_introduced {
        if candidate >= &intro_ver {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_compares_semver_correctly() {
        let v1 = Version::parse("4.17.15");
        let v2 = Version::parse("4.17.19");
        let v3 = Version::parse("v4.17.15");
        let v4 = Version::parse("4.17.15-alpha.1");

        assert_eq!(v1, v3);
        assert!(v1 < v2);
        assert!(v4 < v1);
        assert!(v2 > v4);
    }

    #[test]
    fn evaluates_fixed_event_ranges() {
        let candidate_vuln = Version::parse("4.17.15");
        let candidate_fixed = Version::parse("4.17.19");
        let candidate_safe = Version::parse("4.17.21");

        let events = vec![(Some("0".to_string()), Some("4.17.19".to_string()), None)];

        assert!(is_version_in_range(&candidate_vuln, &events));
        assert!(!is_version_in_range(&candidate_fixed, &events));
        assert!(!is_version_in_range(&candidate_safe, &events));
    }

    #[test]
    fn evaluates_last_affected_event_ranges() {
        let candidate_vuln = Version::parse("1.2.5");
        let candidate_safe = Version::parse("1.2.6");

        let events = vec![(Some("0".to_string()), None, Some("1.2.5".to_string()))];

        assert!(is_version_in_range(&candidate_vuln, &events));
        assert!(!is_version_in_range(&candidate_safe, &events));
    }

    #[test]
    fn handles_different_length_numeric_parts() {
        let v1 = Version::parse("2.20");
        let v2 = Version::parse("2.20.0");
        let v3 = Version::parse("2.20.1");

        assert_eq!(v1, v2);
        assert!(v1 < v3);
    }
}
