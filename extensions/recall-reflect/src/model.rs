use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReflectScopeKind {
    Project,
    Personal,
}

impl ReflectScopeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Personal => "personal",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ReflectFilters {
    pub scope_kind: ReflectScopeKind,
    pub sources: Option<Vec<String>>,
    pub time_range: String,
    pub directory: Option<String>,
    pub repo: Option<String>,
}

impl Default for ReflectFilters {
    fn default() -> Self {
        Self {
            scope_kind: ReflectScopeKind::Project,
            sources: None,
            time_range: String::new(),
            directory: None,
            repo: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SourceSession {
    pub id: String,
    pub source: String,
    pub title: String,
    pub directory: Option<String>,
    pub started_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub messages: Vec<SourceMessage>,
}

#[derive(Clone, Debug)]
pub struct SourceMessage {
    pub role: String,
    pub content: String,
    pub seq: u32,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReflectScope {
    pub kind: ReflectScopeKind,
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
