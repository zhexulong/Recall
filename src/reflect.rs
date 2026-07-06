use anyhow::Result;
use serde::Serialize;

use crate::db::search::{RepoFilter, TimeRange};
use crate::db::store::{SessionListSort, Store};

#[derive(Debug, Clone)]
pub struct ReflectFilters {
    pub sources: Option<Vec<String>>,
    pub time_range: TimeRange,
    pub directory: Option<String>,
    pub repo: Option<RepoFilter>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReflectSummary {
    pub sessions: usize,
    pub earliest_timestamp: Option<i64>,
    pub latest_timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineMoment {
    pub timestamp: i64,
    pub label: String,
    pub session_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Phase {
    pub start_timestamp: i64,
    pub end_timestamp: Option<i64>,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObservedPattern {
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Proposal {
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReflectReport {
    pub summary: ReflectSummary,
    pub phases: Vec<Phase>,
    pub moments: Vec<TimelineMoment>,
    pub observed_patterns: Vec<ObservedPattern>,
    pub proposals: Vec<Proposal>,
    pub coverage_note: Option<String>,
}

pub fn build_reflect_report(store: &Store, filters: &ReflectFilters) -> Result<ReflectReport> {
    let sessions = store.list_indexed_sessions(
        filters.sources.as_deref(),
        filters.time_range,
        filters.directory.as_deref(),
        filters.repo.as_ref(),
        None,
        0,
        SessionListSort::Oldest,
    )?;

    if sessions.is_empty() {
        return Ok(ReflectReport {
            summary: ReflectSummary {
                sessions: 0,
                earliest_timestamp: None,
                latest_timestamp: None,
            },
            phases: Vec::new(),
            moments: Vec::new(),
            observed_patterns: Vec::new(),
            proposals: Vec::new(),
            coverage_note: Some("No sessions matched the reflect scope.".to_string()),
        });
    }

    let earliest = sessions.first().map(|s| s.started_at);
    let latest = sessions.last().map(|s| s.started_at);

    Ok(ReflectReport {
        summary: ReflectSummary {
            sessions: sessions.len(),
            earliest_timestamp: earliest,
            latest_timestamp: latest,
        },
        phases: Vec::new(),
        moments: Vec::new(),
        observed_patterns: Vec::new(),
        proposals: Vec::new(),
        coverage_note: None,
    })
}
