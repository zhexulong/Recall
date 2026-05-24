---
name: update-recall-homebrew-tap
description: Project-specific Recall release workflow. Use after a new samzong/Recall release is published to update samzong/homebrew-tap Formula/recall.rb to the current Cargo.toml version, refresh release asset SHA256 values, push a branch, and open a Homebrew tap PR.
---

# Update Recall Homebrew Tap

Use this skill when Recall has a new GitHub release and the Homebrew tap still points at an older version.

## What this does

The helper script:

1. Reads the Recall version from `Cargo.toml` unless `--version` is provided.
2. Verifies the matching `samzong/Recall` GitHub release exists.
3. Downloads the Homebrew release assets and computes SHA256 checksums:
   - `recall-macos-aarch64.tar.gz`
   - `recall-macos-x86_64.tar.gz`
   - `recall-linux-x86_64.tar.gz`
4. Clones `samzong/homebrew-tap` into a temporary work directory.
5. Updates `Formula/recall.rb` with the new version and checksums.
6. Runs `ruby -c Formula/recall.rb`.
7. Commits, pushes `update-recall-<version>`, and opens a PR.
8. Watches PR checks unless `--no-watch` is passed.

## Default usage

Run from the Recall repository root:

```bash
.agents/skills/update-recall-homebrew-tap/scripts/update-recall-homebrew-tap.sh
```

For a dry run that does not push or open a PR:

```bash
.agents/skills/update-recall-homebrew-tap/scripts/update-recall-homebrew-tap.sh --dry-run --no-watch
```

To update a specific release instead of reading `Cargo.toml`:

```bash
.agents/skills/update-recall-homebrew-tap/scripts/update-recall-homebrew-tap.sh --version 0.2.2
```

## Agent instructions

When invoked:

1. Run the dry run first if the user asks to preview.
2. Otherwise run the default command.
3. Report the PR URL, check status, and whether it merged automatically.
4. If the script says the formula is already current, report that no PR was needed.

Do not edit `samzong/Recall` source code for this workflow. The only target file in the tap is `Formula/recall.rb`.
