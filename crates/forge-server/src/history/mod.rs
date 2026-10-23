//! Multi-branch historical score retention and trend analysis.

use serde::{Deserialize, Serialize};

use crate::db::models::HistoryPoint;
use forge_core::ir::HealthScore;

/// Directional trajectory of codebase health over historical observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    /// Overall health score improved by more than 2 points.
    Improving,
    /// Overall health score degraded by more than 2 points.
    Degrading,
    /// Health score remained within +/- 2 points.
    Stable,
}

/// Aggregate statistical trend over chronological analysis snapshots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryTrend {
    pub repo_id: String,
    pub branch: String,
    pub total_points: usize,
    pub latest_health: Option<u8>,
    pub average_health: f64,
    pub min_health: Option<u8>,
    pub max_health: Option<u8>,
    pub trend_direction: TrendDirection,
    pub health_delta: i32,
    pub architecture_avg: f64,
    pub security_avg: f64,
    pub maintainability_avg: f64,
    pub points: Vec<HistoryPoint>,
}

/// Computes statistical trends across a sequence of chronological history points.
pub fn calculate_history_trend(
    repo_id: &str,
    branch: &str,
    points: Vec<HistoryPoint>,
) -> HistoryTrend {
    if points.is_empty() {
        return HistoryTrend {
            repo_id: repo_id.to_string(),
            branch: branch.to_string(),
            total_points: 0,
            latest_health: None,
            average_health: 0.0,
            min_health: None,
            max_health: None,
            trend_direction: TrendDirection::Stable,
            health_delta: 0,
            architecture_avg: 0.0,
            security_avg: 0.0,
            maintainability_avg: 0.0,
            points: Vec::new(),
        };
    }

    let total_points = points.len();
    let first_health = points.first().map(|p| p.overall_health).unwrap_or(0);
    let latest_health = points.last().map(|p| p.overall_health).unwrap_or(0);
    let health_delta = latest_health as i32 - first_health as i32;

    let trend_direction = if health_delta > 2 {
        TrendDirection::Improving
    } else if health_delta < -2 {
        TrendDirection::Degrading
    } else {
        TrendDirection::Stable
    };

    let mut sum_health = 0u64;
    let mut min_h = u8::MAX;
    let mut max_h = 0u8;

    let mut arch_sum = 0u64;
    let mut arch_count = 0usize;
    let mut sec_sum = 0u64;
    let mut sec_count = 0usize;
    let mut maint_sum = 0u64;
    let mut maint_count = 0usize;

    for point in &points {
        let h = point.overall_health;
        sum_health += h as u64;
        if h < min_h {
            min_h = h;
        }
        if h > max_h {
            max_h = h;
        }

        if let Ok(health_score) = serde_json::from_str::<HealthScore>(&point.health_json) {
            for dim in health_score.dimensions {
                let name = dim.dimension.to_lowercase();
                if name.contains("arch") {
                    arch_sum += dim.score as u64;
                    arch_count += 1;
                } else if name.contains("sec") {
                    sec_sum += dim.score as u64;
                    sec_count += 1;
                } else if name.contains("maint") || name.contains("durab") || name.contains("stand")
                {
                    maint_sum += dim.score as u64;
                    maint_count += 1;
                }
            }
        }
    }

    let average_health = (sum_health as f64) / (total_points as f64);
    let architecture_avg = if arch_count > 0 {
        (arch_sum as f64) / (arch_count as f64)
    } else {
        0.0
    };
    let security_avg = if sec_count > 0 {
        (sec_sum as f64) / (sec_count as f64)
    } else {
        0.0
    };
    let maintainability_avg = if maint_count > 0 {
        (maint_sum as f64) / (maint_count as f64)
    } else {
        0.0
    };

    HistoryTrend {
        repo_id: repo_id.to_string(),
        branch: branch.to_string(),
        total_points,
        latest_health: Some(latest_health),
        average_health,
        min_health: Some(min_h),
        max_health: Some(max_h),
        trend_direction,
        health_delta,
        architecture_avg,
        security_avg,
        maintainability_avg,
        points,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::ir::DimensionScore;

    fn make_test_point(commit: &str, health: u8, arch: u8, sec: u8, maint: u8) -> HistoryPoint {
        let score = HealthScore {
            overall: health,
            dimensions: vec![
                DimensionScore {
                    dimension: "architecture".to_string(),
                    score: arch,
                    weight_pct: 35,
                    reasons: Vec::new(),
                },
                DimensionScore {
                    dimension: "security".to_string(),
                    score: sec,
                    weight_pct: 35,
                    reasons: Vec::new(),
                },
                DimensionScore {
                    dimension: "maintainability".to_string(),
                    score: maint,
                    weight_pct: 30,
                    reasons: Vec::new(),
                },
            ],
        };
        HistoryPoint {
            commit_hash: commit.to_string(),
            branch: "main".to_string(),
            timestamp_secs: 1_700_000_000,
            overall_health: health,
            findings_count: 2,
            health_json: serde_json::to_string(&score).unwrap_or_default(),
        }
    }

    #[test]
    fn test_calculate_history_trend_empty() {
        let trend = calculate_history_trend("repo1", "main", Vec::new());
        assert_eq!(trend.total_points, 0);
        assert_eq!(trend.trend_direction, TrendDirection::Stable);
        assert_eq!(trend.latest_health, None);
        assert_eq!(trend.average_health, 0.0);
    }

    #[test]
    fn test_calculate_history_trend_improving() {
        let points = vec![
            make_test_point("c1", 70, 70, 70, 70),
            make_test_point("c2", 80, 80, 80, 80),
            make_test_point("c3", 85, 85, 85, 85),
        ];

        let trend = calculate_history_trend("repo1", "main", points);
        assert_eq!(trend.total_points, 3);
        assert_eq!(trend.latest_health, Some(85));
        assert_eq!(trend.min_health, Some(70));
        assert_eq!(trend.max_health, Some(85));
        assert_eq!(trend.health_delta, 15);
        assert_eq!(trend.trend_direction, TrendDirection::Improving);
        assert!((trend.average_health - 78.333).abs() < 0.01);
        assert!((trend.architecture_avg - 78.333).abs() < 0.01);
        assert!((trend.security_avg - 78.333).abs() < 0.01);
        assert!((trend.maintainability_avg - 78.333).abs() < 0.01);
    }

    #[test]
    fn test_calculate_history_trend_degrading() {
        let points = vec![
            make_test_point("c1", 90, 90, 90, 90),
            make_test_point("c2", 80, 80, 80, 80),
        ];

        let trend = calculate_history_trend("repo1", "main", points);
        assert_eq!(trend.health_delta, -10);
        assert_eq!(trend.trend_direction, TrendDirection::Degrading);
    }

    #[test]
    fn test_calculate_history_trend_stable() {
        let points = vec![
            make_test_point("c1", 88, 88, 88, 88),
            make_test_point("c2", 89, 89, 89, 89),
        ];

        let trend = calculate_history_trend("repo1", "main", points);
        assert_eq!(trend.health_delta, 1);
        assert_eq!(trend.trend_direction, TrendDirection::Stable);
    }
}
