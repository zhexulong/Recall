use std::fmt::Write;

use crate::model::ReflectReport;
use crate::report::REFLECT_CHUNK_MOMENT_LIMIT;

pub fn render_text(report: &ReflectReport) -> String {
    let mut out = String::new();

    let _ = writeln!(out, "Recall reflect");
    let _ = writeln!(out);

    let _ = writeln!(out, "Scope");
    let _ = writeln!(out, "  Kind: {}", report.scope.kind.as_str());
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

        for moment in phase.moments.iter().take(REFLECT_CHUNK_MOMENT_LIMIT) {
            let time = format_message_time(moment.timestamp);
            let _ = writeln!(
                out,
                "  [{time}] [{role}] [{source}] {title}: {summary}",
                role = moment.role,
                source = moment.source,
                title = moment.session_title,
                summary = moment.summary,
            );
        }
        if phase.moments.len() > REFLECT_CHUNK_MOMENT_LIMIT {
            let _ = writeln!(
                out,
                "  ... and {} more moments",
                phase.moments.len() - REFLECT_CHUNK_MOMENT_LIMIT
            );
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

fn format_message_time(timestamp: i64) -> String {
    timestamp.to_string()
}

#[cfg(test)]
mod tests {
    use crate::model::{ReflectFilters, SourceMessage, SourceSession};
    use crate::render::render_text;
    use crate::report::build_reflect_report;

    fn fixture_session(
        id: &str,
        source: &str,
        title: &str,
        started_at: i64,
        messages: Vec<SourceMessage>,
    ) -> SourceSession {
        SourceSession {
            id: id.to_string(),
            source: source.to_string(),
            title: title.to_string(),
            directory: Some("/tmp/reflect-repo".to_string()),
            started_at: Some(started_at),
            updated_at: None,
            messages,
        }
    }

    fn fixture_message(role: &str, content: &str, seq: u32, timestamp: i64) -> SourceMessage {
        SourceMessage {
            role: role.to_string(),
            content: content.to_string(),
            seq,
            timestamp: Some(timestamp),
        }
    }

    #[test]
    fn reflect_text_output_is_timeline_first() {
        let sessions = vec![fixture_session(
            "s1",
            "codex",
            "Test session",
            1000,
            vec![
                fixture_message("user", "hello world", 0, 1100),
                fixture_message("assistant", "hi there", 1, 1200),
            ],
        )];

        let report = build_reflect_report(sessions, &ReflectFilters::default());
        let text = render_text(&report);

        assert!(text.contains("Recall reflect"), "output must contain header");
        assert!(text.contains("Scope"), "output must contain Scope section");
        assert!(text.contains("Summary"), "output must contain Summary section");
        assert!(text.contains("Timeline"), "output must contain Timeline section");
        assert!(text.contains("Project conversation timeline"), "must include phase title");
        assert!(text.contains("hello world"), "must include user moment content");
        assert!(text.contains("hi there"), "must include assistant moment content");
        assert!(!text.contains("session_events"), "must not contain raw event names");
    }
}
