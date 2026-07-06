use std::collections::BTreeMap;
use std::fmt::Write;

use anyhow::Result;
use clap::ValueEnum;
use serde::Serialize;

use crate::adapters;
use crate::db::search::{RepoFilter, TimeRange};
use crate::db::store::{SessionListSort, Store};
use crate::query::{parse_time_range, resolve_source_filter};
use crate::utils;

pub const REFLECT_CHUNK_MOMENT_LIMIT: usize = 10;

#[derive(Debug, Clone)]
pub struct ReflectFilters {
    pub sources: Option<Vec<String>>,
    pub time_range: TimeRange,
    pub directory: Option<String>,
    pub repo: Option<RepoFilter>,
}

#[derive(Debug, Clone, Copy, Serialize, ValueEnum)]
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
pub struct ConversationChunk {
    pub id: String,
    pub session_id: String,
    pub start_at: i64,
    pub end_at: i64,
    pub moment_ids: Vec<String>,
    pub summary: String,
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
    pub chunks: Vec<ConversationChunk>,
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
            chunks: Vec::new(),
            phases: Vec::new(),
            observed_patterns: Vec::new(),
            proposals: Vec::new(),
            coverage_note: Some("No sessions matched the reflect scope.".to_string()),
        });
    }

    let mut moments: Vec<TimelineMoment> = Vec::new();

    for session in &sessions {
        let messages = store.get_messages(&session.id)?;
        for msg in &messages {
            let role_str = msg.role.as_str().to_string();
            let timestamp = msg.timestamp.unwrap_or(session.started_at + i64::from(msg.seq));
            let summary = compact_content(&msg.content, 180);

            moments.push(TimelineMoment {
                id: format!("{}:{}", session.id, msg.seq),
                timestamp,
                source: session.source.clone(),
                session_id: session.id.clone(),
                session_title: session.title.clone(),
                role: role_str,
                summary,
            });
        }
    }

    moments.sort_by(|a, b| {
        a.timestamp
            .cmp(&b.timestamp)
            .then_with(|| a.session_id.cmp(&b.session_id))
            .then_with(|| a.id.cmp(&b.id))
    });

    let timeline_moments_count = moments.len();

    let mut sessions_by_id: BTreeMap<String, Vec<&TimelineMoment>> = BTreeMap::new();
    for moment in &moments {
        sessions_by_id.entry(moment.session_id.clone()).or_default().push(moment);
    }

    let mut chunks: Vec<ConversationChunk> = Vec::new();
    for (session_id, session_moments) in &sessions_by_id {
        for (chunk_idx, chunk_moments) in
            session_moments.chunks(REFLECT_CHUNK_MOMENT_LIMIT).enumerate()
        {
            if chunk_moments.is_empty() {
                continue;
            }
            let start_at = chunk_moments.first().map(|m| m.timestamp).unwrap_or(0);
            let end_at = chunk_moments.last().map(|m| m.timestamp).unwrap_or(0);
            let moment_ids: Vec<String> = chunk_moments.iter().map(|m| m.id.clone()).collect();
            let count = chunk_moments.len();
            chunks.push(ConversationChunk {
                id: format!("{}:chunk-{}", session_id, chunk_idx + 1),
                session_id: session_id.clone(),
                start_at,
                end_at,
                moment_ids,
                summary: format!("{} conversation moments from {}.", count, session_id),
            });
        }
    }

    chunks.sort_by(|a, b| a.start_at.cmp(&b.start_at).then_with(|| a.id.cmp(&b.id)));

    let observed_patterns = detect_observed_patterns(&moments);

    let mut phases = Vec::new();

    if !moments.is_empty() {
        let start_at = moments.first().map(|m| m.timestamp).unwrap_or(0);
        let end_at = moments.last().map(|m| m.timestamp).unwrap_or(0);
        let session_ids: std::collections::HashSet<&str> =
            moments.iter().map(|m| m.session_id.as_str()).collect();

        phases.push(TimelinePhase {
            id: "phase-1".to_string(),
            title: "Project conversation timeline".to_string(),
            start_at,
            end_at,
            summary: format!(
                "{} conversation moments in {} chunks across {} sessions.",
                timeline_moments_count,
                chunks.len(),
                session_ids.len()
            ),
            moments,
        });
    }

    Ok(ReflectReport {
        scope,
        summary: ReflectSummary {
            sessions: sessions.len(),
            timeline_moments: timeline_moments_count,
            phases: phases.len(),
        },
        chunks,
        phases,
        observed_patterns,
        proposals: Vec::new(),
        coverage_note: None,
    })
}

fn detect_observed_patterns(moments: &[TimelineMoment]) -> Vec<ObservedPattern> {
    let scope_signals = ["scope", "don't expand", "do not expand", "keep it small", "不要扩大"];

    let matched: Vec<&str> = moments
        .iter()
        .filter(|m| {
            let lower = m.summary.to_lowercase();
            scope_signals.iter().any(|sig| lower.contains(&sig.to_lowercase()))
        })
        .map(|m| m.id.as_str())
        .collect();

    if matched.len() >= 2 {
        vec![ObservedPattern {
            id: "pattern-scope-boundary".to_string(),
            summary: "Scope boundary reminders appeared in multiple timeline moments."
                .to_string(),
            timeline_moments: matched.into_iter().map(String::from).collect(),
            discussion_prompt:
                "Is this a real workflow issue worth calibrating, or are these unrelated scope reminders?"
                    .to_string(),
        }]
    } else {
        Vec::new()
    }
}

fn compact_content(content: &str, max_chars: usize) -> String {
    let collapsed: String = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.len() <= max_chars {
        collapsed
    } else {
        let mut truncated: String = collapsed.chars().take(max_chars).collect();
        truncated.push_str("...");
        truncated
    }
}

pub fn render_text(report: &ReflectReport) -> String {
    let mut out = String::new();

    let _ = writeln!(out, "Recall reflect");
    let _ = writeln!(out);

    let _ = writeln!(out, "Scope");
    let _ = writeln!(out, "  Project: {}", report.scope.project.as_deref().unwrap_or("-"));
    let _ = writeln!(out, "  Repo: {}", report.scope.repo.as_deref().unwrap_or("-"));
    let _ = writeln!(out, "  Time: {}", report.scope.time_range);
    if !report.scope.sources.is_empty() {
        let _ = writeln!(out, "  Sources: {}", report.scope.sources.join(", "));
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "Summary");
    let _ = writeln!(out, "  Sessions: {}", report.summary.sessions);
    let _ = writeln!(out, "  Moments: {}", report.summary.timeline_moments);
    let _ = writeln!(out, "  Phases: {}", report.summary.phases);
    let _ = writeln!(out);

    if let Some(note) = &report.coverage_note {
        let _ = writeln!(out, "Note: {note}");
        let _ = writeln!(out);
        return out;
    }

    for phase in &report.phases {
        let _ = writeln!(out, "Timeline: {}", phase.title);
        let _ = writeln!(out, "  {}", phase.summary);
        let _ = writeln!(out);

        let max_moments = REFLECT_CHUNK_MOMENT_LIMIT;
        for moment in phase.moments.iter().take(max_moments) {
            let time = utils::format_message_time(Some(moment.timestamp));
            let _ = writeln!(
                out,
                "  [{time}] [{role}] [{source}] {title}: {summary}",
                time = time,
                role = moment.role,
                source = moment.source,
                title = moment.session_title,
                summary = moment.summary,
            );
        }
        if phase.moments.len() > max_moments {
            let _ = writeln!(out, "  ... and {} more moments", phase.moments.len() - max_moments);
        }
        let _ = writeln!(out);
    }

    if !report.observed_patterns.is_empty() {
        let _ = writeln!(out, "Discussion Prompts");
        for pattern in &report.observed_patterns {
            let _ = writeln!(out, "  - {}", pattern.discussion_prompt);
        }
        let _ = writeln!(out);
    }

    out
}

pub fn run_cli(
    format: ReflectFormat,
    source_filter: Option<&str>,
    time_filter: Option<&str>,
    project_filter: Option<&str>,
    repo_filter: Option<&str>,
    sync: bool,
) -> Result<()> {
    if sync {
        crate::sync::run_cli(false, false, source_filter)?;
    }

    let store = Store::open()?;
    let sources = adapters::source_labels();
    let resolved_sources = resolve_source_filter(source_filter, &sources)?;
    let time_range = parse_time_range(time_filter);
    let (directory, repo) = store.resolve_project_repo_filters(project_filter, repo_filter)?;

    let filters = ReflectFilters { sources: resolved_sources, time_range, directory, repo };
    let report = build_reflect_report(&store, &filters)?;

    match format {
        ReflectFormat::Text => {
            print!("{}", render_text(&report));
        }
        ReflectFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    Ok(())
}
