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
pub enum ReflectFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReflectScope {
    pub project: Option<String>,
    pub repo: Option<String>,
    pub time_range: String,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReflectSummary {
    pub sessions: usize,
    pub timeline_moments: usize,
    pub phases: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelinePhase {
    pub id: String,
    pub title: String,
    pub start_at: i64,
    pub end_at: i64,
    pub summary: String,
    pub moments: Vec<TimelineMoment>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineMoment {
    pub id: String,
    pub timestamp: i64,
    pub source: String,
    pub session_id: String,
    pub session_title: String,
    pub role: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObservedPattern {
    pub id: String,
    pub summary: String,
    pub timeline_moments: Vec<String>,
    pub discussion_prompt: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReflectProposalStub {
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReflectReport {
    pub scope: ReflectScope,
    pub summary: ReflectSummary,
    pub phases: Vec<TimelinePhase>,
    pub observed_patterns: Vec<ObservedPattern>,
    pub proposals: Vec<ReflectProposalStub>,
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

    let scope = ReflectScope {
        project: filters.directory.clone(),
        repo: filters.repo.as_ref().map(|r| format!("{:?}", r)),
        time_range: format!("{:?}", filters.time_range),
        sources: filters.sources.clone().unwrap_or_default(),
    };

    if sessions.is_empty() {
        return Ok(ReflectReport {
            scope,
            summary: ReflectSummary { sessions: 0, timeline_moments: 0, phases: 0 },
            phases: Vec::new(),
            observed_patterns: Vec::new(),
            proposals: Vec::new(),
            coverage_note: Some("No sessions matched the reflect scope.".to_string()),
        });
    }

    Ok(ReflectReport {
        scope,
        summary: ReflectSummary { sessions: sessions.len(), timeline_moments: 0, phases: 0 },
        phases: Vec::new(),
        observed_patterns: Vec::new(),
        proposals: Vec::new(),
        coverage_note: None,
    })
}
