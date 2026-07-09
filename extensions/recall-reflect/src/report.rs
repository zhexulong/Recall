use std::collections::{BTreeMap, HashSet};

use crate::model::{
    ConversationChunk, ReflectFilters, ReflectReport, ReflectScope, ReflectScopeKind,
    ReflectSummary, SourceSession, TimelineMoment, TimelinePhase,
};
use crate::patterns::detect_observed_patterns;

pub const REFLECT_CHUNK_MOMENT_LIMIT: usize = 10;

pub fn build_reflect_report(
    sessions: Vec<SourceSession>,
    filters: &ReflectFilters,
) -> ReflectReport {
    let scope = ReflectScope {
        kind: filters.scope_kind,
        project: filters.directory.clone(),
        repo: filters.repo.clone(),
        time_range: if filters.time_range.is_empty() {
            "All".to_string()
        } else {
            filters.time_range.clone()
        },
        sources: filters.sources.clone().unwrap_or_default(),
    };

    if sessions.is_empty() {
        return ReflectReport {
            scope,
            summary: ReflectSummary { sessions: 0, timeline_moments: 0, phases: 0 },
            chunks: Vec::new(),
            phases: Vec::new(),
            observed_patterns: Vec::new(),
            proposals: Vec::new(),
            coverage_note: Some("No sessions matched the reflect scope.".to_string()),
        };
    }

    let session_count = sessions.len();
    let mut moments = Vec::new();

    for session in &sessions {
        let _ = (&session.directory, &session.updated_at);
        for msg in &session.messages {
            let timestamp = msg.timestamp.unwrap_or_else(|| {
                session
                    .started_at
                    .or(session.updated_at)
                    .unwrap_or(0)
                    .saturating_add(i64::from(msg.seq))
            });

            let Some(cleaned) = sanitize_conversation_content(&msg.content) else {
                continue;
            };

            moments.push(TimelineMoment {
                id: format!("{}:{}", session.id, msg.seq),
                timestamp,
                source: session.source.clone(),
                session_id: session.id.clone(),
                session_title: session.title.clone(),
                role: msg.role.clone(),
                summary: compact_content(&cleaned, 180),
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

    let mut chunks = Vec::new();
    for (session_id, session_moments) in &sessions_by_id {
        for (chunk_idx, chunk_moments) in session_moments
            .chunks(REFLECT_CHUNK_MOMENT_LIMIT)
            .enumerate()
            .filter(|(_, chunk)| !chunk.is_empty())
        {
            let start_at = chunk_moments.first().map(|m| m.timestamp).unwrap_or(0);
            let end_at = chunk_moments.last().map(|m| m.timestamp).unwrap_or(0);
            let moment_ids = chunk_moments.iter().map(|m| m.id.clone()).collect();
            chunks.push(ConversationChunk {
                id: format!("{}:chunk-{}", session_id, chunk_idx + 1),
                session_id: session_id.clone(),
                start_at,
                end_at,
                moment_ids,
                summary: format!(
                    "{} conversation moments from {}.",
                    chunk_moments.len(),
                    session_id
                ),
            });
        }
    }
    chunks.sort_by(|a, b| a.start_at.cmp(&b.start_at).then_with(|| a.id.cmp(&b.id)));

    let observed_patterns = detect_observed_patterns(&moments);
    let mut phases = Vec::new();

    if !moments.is_empty() {
        let start_at = moments.first().map(|m| m.timestamp).unwrap_or(0);
        let end_at = moments.last().map(|m| m.timestamp).unwrap_or(0);
        let session_ids: HashSet<&str> = moments.iter().map(|m| m.session_id.as_str()).collect();

        phases.push(TimelinePhase {
            id: "phase-1".to_string(),
            title: timeline_title(filters.scope_kind).to_string(),
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

    ReflectReport {
        scope,
        summary: ReflectSummary {
            sessions: session_count,
            timeline_moments: timeline_moments_count,
            phases: phases.len(),
        },
        chunks,
        phases,
        observed_patterns,
        proposals: Vec::new(),
        coverage_note: None,
    }
}

fn timeline_title(scope_kind: ReflectScopeKind) -> &'static str {
    match scope_kind {
        ReflectScopeKind::Project => "Project conversation timeline",
        ReflectScopeKind::Personal => "Personal conversation timeline",
    }
}

fn compact_content(content: &str, max_chars: usize) -> String {
    let collapsed = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.len() <= max_chars {
        collapsed
    } else {
        let mut truncated: String = collapsed.chars().take(max_chars).collect();
        truncated.push_str("...");
        truncated
    }
}

fn sanitize_conversation_content(content: &str) -> Option<String> {
    if is_low_level_transcript_log(content) {
        return None;
    }

    let tool_names = [
        "Bash",
        "Read",
        "Write",
        "Edit",
        "Grep",
        "Glob",
        "LS",
        "TodoWrite",
        "Task",
        "WebFetch",
        "WebSearch",
    ];

    let mut cut_pos = content.len();
    for name in &tool_names {
        let marker = format!("[{name}]");
        if let Some(pos) = content.find(&marker)
            && pos > 0
            && pos < cut_pos
        {
            cut_pos = pos;
        }
    }

    if cut_pos < content.len() {
        let sanitized = content[..cut_pos].trim_end();
        if sanitized.is_empty() {
            return None;
        }
        return Some(sanitized.to_string());
    }

    Some(content.to_string())
}

fn is_low_level_transcript_log(content: &str) -> bool {
    let tool_prefixes = [
        "[Bash]",
        "[Read]",
        "[Write]",
        "[Edit]",
        "[Grep]",
        "[Glob]",
        "[LS]",
        "[TodoWrite]",
        "[Task]",
        "[WebFetch]",
        "[WebSearch]",
    ];

    if tool_prefixes.iter().any(|p| content.starts_with(p)) {
        return true;
    }

    let envelope_prefixes =
        ["<command-message>", "<command-name>", "<local-command-stdout>", "<local-command-stderr>"];

    if envelope_prefixes.iter().any(|p| content.starts_with(p)) {
        return true;
    }

    if content.starts_with("The file ") && content.contains(" has been updated successfully.") {
        return true;
    }

    if content.starts_with("File created successfully at:") {
        return true;
    }

    if content.starts_with("Tool execution aborted") {
        return true;
    }

    if content.starts_with("(Bash completed with no output)") {
        return true;
    }

    if content.starts_with("The user doesn't want to proceed with this tool use") {
        return true;
    }

    if content.starts_with("[Request interrupted by user for tool use]") {
        return true;
    }

    is_line_numbered_file_dump(content)
}

fn is_line_numbered_file_dump(content: &str) -> bool {
    let trimmed = content.trim_start();
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_digit() {
        return false;
    }
    for i in 1..bytes.len().min(6) {
        let b = bytes[i];
        if b == b'#' || (b.is_ascii_whitespace() && i + 1 < bytes.len() && bytes[i + 1] == b'#') {
            return true;
        }
        if !b.is_ascii_digit() && !b.is_ascii_whitespace() {
            break;
        }
    }
    false
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
    fn reflect_empty_scope_returns_coverage_note() {
        let report = build_reflect_report(Vec::new(), &ReflectFilters::default());

        assert_eq!(report.coverage_note.as_deref(), Some("No sessions matched the reflect scope."));
        assert_eq!(report.summary.sessions, 0);
        assert_eq!(report.summary.timeline_moments, 0);
        assert_eq!(report.summary.phases, 0);
        assert!(report.phases.is_empty());
        assert!(report.observed_patterns.is_empty());
        assert!(report.proposals.is_empty());
        assert!(report.chunks.is_empty());
    }

    #[test]
    fn reflect_report_includes_project_scope_kind_by_default() {
        let sessions = vec![fixture_session(
            "s1",
            "codex",
            "Scoped session",
            1000,
            vec![fixture_message("user", "hello", 0, 1100)],
        )];
        let filters = ReflectFilters {
            directory: Some("/tmp/reflect-repo".to_string()),
            ..ReflectFilters::default()
        };

        let report = build_reflect_report(sessions, &filters);

        assert_eq!(report.scope.kind.as_str(), "project");
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["scope"]["kind"], "project");
    }

    #[test]
    fn reflect_report_labels_personal_timeline() {
        let sessions = vec![fixture_session(
            "s1",
            "codex",
            "Personal session",
            1000,
            vec![fixture_message("user", "hello", 0, 1100)],
        )];
        let filters = ReflectFilters {
            scope_kind: crate::model::ReflectScopeKind::Personal,
            time_range: "30d".to_string(),
            ..ReflectFilters::default()
        };

        let report = build_reflect_report(sessions, &filters);

        assert_eq!(report.phases[0].title, "Personal conversation timeline");
    }

    #[test]
    fn reflect_builds_timeline_across_sessions() {
        let sessions = vec![
            fixture_session(
                "s1",
                "codex",
                "Codex session",
                1000,
                vec![
                    fixture_message("user", "hello", 0, 1000),
                    fixture_message("assistant", "hi there", 1, 1100),
                ],
            ),
            fixture_session(
                "s2",
                "opencode",
                "OpenCode session",
                500,
                vec![
                    fixture_message("user", "how fix parser", 0, 600),
                    fixture_message("assistant", "check imports", 1, 700),
                ],
            ),
        ];

        let report = build_reflect_report(sessions, &ReflectFilters::default());

        assert_eq!(report.summary.sessions, 2);
        assert_eq!(report.summary.timeline_moments, 4);
        assert_eq!(report.summary.phases, 1);
        assert_eq!(report.phases.len(), 1, "should have one timeline phase");

        let phase = &report.phases[0];
        assert_eq!(phase.id, "phase-1");
        assert_eq!(phase.title, "Project conversation timeline");
        assert_eq!(phase.moments.len(), 4);
        assert_eq!(phase.moments[0].id, "s2:0");
        assert_eq!(phase.moments[0].timestamp, 600);
        assert_eq!(phase.moments[0].role, "user");
        assert_eq!(phase.moments[0].session_id, "s2");
        assert_eq!(phase.moments[0].source, "opencode");
        assert_eq!(phase.moments[0].session_title, "OpenCode session");
        assert_eq!(phase.moments[1].id, "s2:1");
        assert_eq!(phase.moments[1].timestamp, 700);
        assert_eq!(phase.moments[1].role, "assistant");
        assert_eq!(phase.moments[2].id, "s1:0");
        assert_eq!(phase.moments[2].timestamp, 1000);
        assert_eq!(phase.moments[3].id, "s1:1");
        assert_eq!(phase.moments[3].timestamp, 1100);

        for moment in &phase.moments {
            assert!(!moment.summary.is_empty(), "moment {} should have summary", moment.id);
        }
        assert_eq!(phase.start_at, 600);
        assert_eq!(phase.end_at, 1100);
        assert!(phase.summary.contains("conversation moments"));
        assert!(phase.summary.contains("2 sessions"));
        assert!(report.observed_patterns.is_empty());
        assert!(report.proposals.is_empty());
        assert!(report.coverage_note.is_none());
    }

    #[test]
    fn reflect_chunks_long_sessions_before_project_summary() {
        let messages = (0..25)
            .map(|i| {
                let role = if i % 2 == 0 { "user" } else { "assistant" };
                fixture_message(role, &format!("message {i}"), i, 1000 + i64::from(i) * 100)
            })
            .collect();
        let sessions = vec![fixture_session("s1", "codex", "Long session", 1000, messages)];

        let report = build_reflect_report(sessions, &ReflectFilters::default());

        assert!(report.chunks.len() > 1, "25 moments should produce multiple chunks");
        assert_eq!(report.summary.timeline_moments, 25);
        assert!(report.phases[0].summary.contains("chunks"));
    }

    #[test]
    fn reflect_scope_pattern_is_discussion_prompt_only() {
        let sessions = vec![
            fixture_session(
                "s1",
                "codex",
                "Scope session one",
                1000,
                vec![
                    fixture_message("user", "Keep it small; do not expand scope.", 0, 1100),
                    fixture_message("assistant", "Understood, staying focused.", 1, 1200),
                ],
            ),
            fixture_session(
                "s2",
                "codex",
                "Scope session two",
                2000,
                vec![
                    fixture_message("user", "Again, don't expand scope this time.", 0, 2100),
                    fixture_message("assistant", "Got it.", 1, 2200),
                ],
            ),
        ];

        let report = build_reflect_report(sessions, &ReflectFilters::default());

        assert_eq!(report.observed_patterns.len(), 1);
        let pattern = &report.observed_patterns[0];
        assert_eq!(pattern.id, "pattern-scope-boundary");
        assert!(pattern.summary.to_lowercase().contains("scope"));
        assert_eq!(pattern.timeline_moments.len(), 2);
        assert!(pattern.discussion_prompt.to_lowercase().contains("workflow issue"));
        assert!(report.proposals.is_empty(), "proposals must remain empty");
    }

    #[test]
    fn reflect_scope_pattern_requires_repeated_signal() {
        let sessions = vec![fixture_session(
            "s1",
            "codex",
            "Single scope session",
            1000,
            vec![
                fixture_message("user", "Please do not expand scope.", 0, 1100),
                fixture_message("assistant", "Got it.", 1, 1200),
            ],
        )];

        let report = build_reflect_report(sessions, &ReflectFilters::default());

        assert!(report.observed_patterns.is_empty());
    }

    #[test]
    fn reflect_excludes_low_level_transcript_logs_by_default() {
        let sessions = vec![fixture_session(
            "s1",
            "opencode",
            "Mixed session",
            1000,
            vec![
                fixture_message("user", "Please review the timeline design", 0, 1100),
                fixture_message("assistant", "I will review the design at a high level", 1, 1200),
                fixture_message("assistant", "[Bash] {\"command\":\"git status\"}", 2, 1300),
                fixture_message(
                    "assistant",
                    "[Read] {\"file_path\":\"docs/extensions/reflect.md\"}",
                    3,
                    1400,
                ),
                fixture_message("assistant", "[Write] {\"file_path\":\"x\"}", 4, 1500),
                fixture_message(
                    "assistant",
                    "<command-message>ui-ux-pro-max</command-message>",
                    5,
                    1600,
                ),
                fixture_message(
                    "assistant",
                    "<local-command-stdout>Copied</local-command-stdout>",
                    6,
                    1700,
                ),
                fixture_message(
                    "assistant",
                    "The file /tmp/example.md has been updated successfully.",
                    7,
                    1800,
                ),
                fixture_message("assistant", "(Bash completed with no output)", 8, 1900),
                fixture_message(
                    "assistant",
                    "I ran a quick bash script to verify the read paths and write output.",
                    9,
                    2000,
                ),
            ],
        )];

        let report = build_reflect_report(sessions, &ReflectFilters::default());
        let surviving_summaries: Vec<&str> =
            report.phases[0].moments.iter().map(|m| m.summary.as_str()).collect();

        assert_eq!(surviving_summaries.len(), 3);
        assert!(surviving_summaries.iter().any(|s| s.contains("review the timeline")));
        assert!(surviving_summaries.iter().any(|s| s.contains("review the design")));
        assert!(surviving_summaries.iter().any(|s| s.contains("bash script")));
        for summary in &surviving_summaries {
            assert!(!summary.starts_with("[Bash]"), "tool log prefix leaked: {summary}");
            assert!(!summary.starts_with("[Read]"), "tool log prefix leaked: {summary}");
            assert!(!summary.starts_with("[Write]"), "tool log prefix leaked: {summary}");
            assert!(
                !summary.starts_with("<command-message>"),
                "command envelope leaked: {summary}"
            );
            assert!(
                !summary.starts_with("<local-command-stdout>"),
                "stdout envelope leaked: {summary}"
            );
            assert!(!summary.starts_with("(Bash completed"), "bash completion leaked: {summary}");
        }
        let text = render_text(&report);
        assert!(!text.contains("[Bash]"));
        assert!(!text.contains("[Read]"));
        assert!(!text.contains("<command-message>"));
        assert!(!text.contains("<local-command-stdout>"));
    }

    #[test]
    fn reflect_sanitizes_inline_tool_artifacts() {
        let sessions = vec![fixture_session(
            "s1",
            "opencode",
            "Inline artifacts session",
            1000,
            vec![
                fixture_message(
                    "assistant",
                    "I'll review the docs. [Read] {\"file_path\":\"docs/extensions/reflect.md\"}",
                    0,
                    1100,
                ),
                fixture_message(
                    "user",
                    "The user doesn't want to proceed with this tool use...",
                    1,
                    1200,
                ),
                fixture_message("user", "[Request interrupted by user for tool use]", 2, 1300),
                fixture_message("user", "1 # Heading 2 3 content from file", 3, 1400),
                fixture_message("assistant", "I ran a bash script to read the file", 4, 1500),
                fixture_message("user", "Please check the output", 5, 1600),
            ],
        )];

        let report = build_reflect_report(sessions, &ReflectFilters::default());
        let surviving_summaries: Vec<&str> =
            report.phases[0].moments.iter().map(|m| m.summary.as_str()).collect();

        assert_eq!(surviving_summaries.len(), 3);
        let sanitized = surviving_summaries
            .iter()
            .find(|s| s.contains("review the docs"))
            .expect("sanitized inline-tool message must survive");
        assert!(!sanitized.contains("[Read]"));
        assert!(!sanitized.contains("file_path"));
        assert!(!sanitized.contains("{\""));
        for summary in &surviving_summaries {
            assert!(!summary.contains("doesn't want to proceed"));
            assert!(!summary.contains("Request interrupted"));
            assert!(!summary.starts_with("1 #"));
        }
        assert!(surviving_summaries.iter().any(|s| s.contains("bash script")));
        assert!(surviving_summaries.iter().any(|s| s.contains("check the output")));

        let text = render_text(&report);
        assert!(!text.contains("[Read]"));
        assert!(!text.contains("file_path"));
        assert!(!text.contains("doesn't want to proceed"));
        assert!(!text.contains("Request interrupted"));
        assert!(text.contains("review the docs"));
    }
}
