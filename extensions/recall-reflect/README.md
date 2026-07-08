# recall-reflect

Official Recall extension for timeline-first workflow reflection.

It consumes Recall through CLI JSON/JSONL output and does not read Recall's SQLite database or Rust internals.

When no `--project` or `--repo` is provided, `recall-reflect` scopes to the current git repository root. Outside a git worktree, pass `--project` or `--repo` explicitly.
