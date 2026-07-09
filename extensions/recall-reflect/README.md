# recall-reflect

Official Recall extension for timeline-first workflow reflection.

It consumes Recall through CLI JSON/JSONL output and does not read Recall's SQLite database or Rust internals.

When no `--project`, `--repo`, or `--personal` scope is provided,
`recall-reflect` scopes to the current git repository root inside a git
worktree. Outside a git worktree, it defaults to personal reflection over the
recent `30d` window. Pass `--personal` to force personal reflection inside a
repository.
