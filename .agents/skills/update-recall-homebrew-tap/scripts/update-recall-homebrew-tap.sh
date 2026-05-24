#!/usr/bin/env bash
set -euo pipefail

SOURCE_REPO="${SOURCE_REPO:-samzong/Recall}"
TAP_REPO="${TAP_REPO:-samzong/homebrew-tap}"
BASE_BRANCH="${BASE_BRANCH:-main}"
FORMULA_PATH="${FORMULA_PATH:-Formula/recall.rb}"
VERSION=""
BRANCH=""
DRY_RUN=0
WATCH=1
KEEP_WORKDIR=0

ASSETS=(
  "recall-macos-aarch64.tar.gz"
  "recall-macos-x86_64.tar.gz"
  "recall-linux-x86_64.tar.gz"
)

usage() {
  cat <<'USAGE'
Update samzong/homebrew-tap Formula/recall.rb for the current Recall release.

Usage:
  update-recall-homebrew-tap.sh [options]

Options:
  --version VERSION       Recall version to publish (default: read package.version from Cargo.toml)
  --branch BRANCH         Branch name for the tap PR (default: update-recall-<version>)
  --source-repo OWNER/REPO
                          Source release repo (default: samzong/Recall)
  --tap-repo OWNER/REPO   Homebrew tap repo (default: samzong/homebrew-tap)
  --base BRANCH           Tap base branch (default: main)
  --dry-run               Update and validate in a temp clone, but do not commit, push, or open a PR
  --no-watch              Do not wait for PR checks after creating the PR
  --keep-workdir          Keep the temporary working directory for inspection
  -h, --help              Show this help

Examples:
  .agents/skills/update-recall-homebrew-tap/scripts/update-recall-homebrew-tap.sh
  .agents/skills/update-recall-homebrew-tap/scripts/update-recall-homebrew-tap.sh --dry-run --no-watch
  .agents/skills/update-recall-homebrew-tap/scripts/update-recall-homebrew-tap.sh --version 0.2.2
USAGE
}

die() {
  echo "error: $*" >&2
  exit 1
}

need() {
  command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

retry() {
  local attempts="$1"
  shift
  local attempt=1
  local delay=2
  local status=0

  until "$@"; do
    status=$?
    if [[ "$attempt" -ge "$attempts" ]]; then
      return "$status"
    fi
    echo "Command failed, retrying in ${delay}s (${attempt}/${attempts}): $*" >&2
    sleep "$delay"
    attempt=$((attempt + 1))
    delay=$((delay * 2))
  done
}

sha256_file() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    die "required command not found: shasum or sha256sum"
  fi
}

read_cargo_version() {
  local repo_root="$1"
  awk '
    /^\[package\]/ { in_package = 1; next }
    /^\[/ && in_package { exit }
    in_package && /^[[:space:]]*version[[:space:]]*=/ { print; exit }
  ' "$repo_root/Cargo.toml" | sed -E 's/.*"([^"]+)".*/\1/'
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      [[ $# -ge 2 ]] || die "--version requires a value"
      VERSION="$2"
      shift 2
      ;;
    --branch)
      [[ $# -ge 2 ]] || die "--branch requires a value"
      BRANCH="$2"
      shift 2
      ;;
    --source-repo)
      [[ $# -ge 2 ]] || die "--source-repo requires a value"
      SOURCE_REPO="$2"
      shift 2
      ;;
    --tap-repo)
      [[ $# -ge 2 ]] || die "--tap-repo requires a value"
      TAP_REPO="$2"
      shift 2
      ;;
    --base)
      [[ $# -ge 2 ]] || die "--base requires a value"
      BASE_BRANCH="$2"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --no-watch)
      WATCH=0
      shift
      ;;
    --keep-workdir)
      KEEP_WORKDIR=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown option: $1"
      ;;
  esac
done

need git
need gh
need ruby
need awk
need sed
need python3

gh auth status >/dev/null 2>&1 || die "gh is not authenticated; run: gh auth login"

REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
[[ -f "$REPO_ROOT/Cargo.toml" ]] || die "Cargo.toml not found; run this from the Recall repository"

if [[ -z "$VERSION" ]]; then
  VERSION=$(read_cargo_version "$REPO_ROOT")
fi
[[ -n "$VERSION" ]] || die "could not determine Recall version"
VERSION="${VERSION#v}"
TAG="v${VERSION}"
BRANCH="${BRANCH:-update-recall-${VERSION}}"

case "$BRANCH" in
  *[!A-Za-z0-9._/-]*) die "branch contains unsupported characters: $BRANCH" ;;
esac

echo "Recall release: ${SOURCE_REPO}@${TAG}"
echo "Homebrew tap:   ${TAP_REPO}:${BASE_BRANCH}"
echo "PR branch:      ${BRANCH}"

if ! retry 3 gh release view "$TAG" --repo "$SOURCE_REPO" >/dev/null 2>&1; then
  die "GitHub release not found: ${SOURCE_REPO}@${TAG}"
fi

release_is_draft=$(retry 3 gh release view "$TAG" --repo "$SOURCE_REPO" --json isDraft --jq '.isDraft')
if [[ "$release_is_draft" == "true" ]]; then
  die "release is still a draft: ${SOURCE_REPO}@${TAG}"
fi

WORKDIR=$(mktemp -d "${TMPDIR:-/tmp}/recall-homebrew-tap.XXXXXX")
cleanup() {
  if [[ "$KEEP_WORKDIR" -eq 1 ]]; then
    echo "Kept workdir: $WORKDIR"
  else
    rm -rf "$WORKDIR"
  fi
}
trap cleanup EXIT

ASSET_DIR="$WORKDIR/assets"
TAP_DIR="$WORKDIR/tap"
mkdir -p "$ASSET_DIR"

download_asset() {
  local asset="$1"
  rm -f "$ASSET_DIR/$asset"
  gh release download "$TAG" --repo "$SOURCE_REPO" --dir "$ASSET_DIR" --pattern "$asset"
}

SHA_MACOS_AARCH64=""
SHA_MACOS_X86_64=""
SHA_LINUX_X86_64=""

for asset in "${ASSETS[@]}"; do
  echo "Downloading asset: $asset"
  retry 3 download_asset "$asset"
  asset_path="$ASSET_DIR/$asset"
  [[ -s "$asset_path" ]] || die "release asset missing or empty: $asset"
  asset_sha=$(sha256_file "$asset_path")
  case "$asset" in
    recall-macos-aarch64.tar.gz) SHA_MACOS_AARCH64="$asset_sha" ;;
    recall-macos-x86_64.tar.gz) SHA_MACOS_X86_64="$asset_sha" ;;
    recall-linux-x86_64.tar.gz) SHA_LINUX_X86_64="$asset_sha" ;;
    *) die "unexpected asset: $asset" ;;
  esac
done

echo "Computed SHA256 checksums:"
echo "  recall-macos-aarch64.tar.gz: $SHA_MACOS_AARCH64"
echo "  recall-macos-x86_64.tar.gz: $SHA_MACOS_X86_64"
echo "  recall-linux-x86_64.tar.gz: $SHA_LINUX_X86_64"

clone_tap() {
  rm -rf "$TAP_DIR"
  git clone --quiet --branch "$BASE_BRANCH" --single-branch "https://github.com/${TAP_REPO}.git" "$TAP_DIR"
}

echo "Cloning tap into temp workdir"
retry 3 clone_tap
cd "$TAP_DIR"
git checkout --quiet -b "$BRANCH"

[[ -f "$FORMULA_PATH" ]] || die "formula not found in tap: $FORMULA_PATH"
OLD_VERSION=$(sed -nE 's/^[[:space:]]*version[[:space:]]+"([^"]+)".*/\1/p' "$FORMULA_PATH" | head -n 1)

RECALL_VERSION="$VERSION" \
SHA_MACOS_AARCH64="$SHA_MACOS_AARCH64" \
SHA_MACOS_X86_64="$SHA_MACOS_X86_64" \
SHA_LINUX_X86_64="$SHA_LINUX_X86_64" \
python3 - "$FORMULA_PATH" <<'PY'
import os
import pathlib
import re
import sys

formula = pathlib.Path(sys.argv[1])
content = formula.read_text()
version = os.environ["RECALL_VERSION"]
checksums = {
    "recall-macos-aarch64.tar.gz": os.environ["SHA_MACOS_AARCH64"],
    "recall-macos-x86_64.tar.gz": os.environ["SHA_MACOS_X86_64"],
    "recall-linux-x86_64.tar.gz": os.environ["SHA_LINUX_X86_64"],
}


def replace_once(text: str, pattern: str, replacement: str, label: str) -> str:
    updated, count = re.subn(pattern, replacement, text, count=1, flags=re.MULTILINE)
    if count != 1:
        raise SystemExit(f"could not update {label}; expected exactly one match, found {count}")
    return updated


content = replace_once(
    content,
    r'(^\s*version\s+")([^"]+)(")',
    rf'\g<1>{version}\g<3>',
    "formula version",
)

for asset, sha256 in checksums.items():
    content = replace_once(
        content,
        rf'(url\s+"[^"]*{re.escape(asset)}"\n\s*sha256\s+")([0-9a-fA-F]{{64}})(")',
        rf'\g<1>{sha256}\g<3>',
        asset,
    )

formula.write_text(content)
PY

ruby -c "$FORMULA_PATH"

if git diff --quiet -- "$FORMULA_PATH"; then
  echo "Formula already matches Recall ${VERSION}; no PR needed."
  exit 0
fi

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "Dry run: formula diff follows. No commit, push, or PR will be created."
  git diff -- "$FORMULA_PATH"
  exit 0
fi

if git ls-remote --exit-code --heads origin "$BRANCH" >/dev/null 2>&1; then
  existing_pr=$(gh pr list --repo "$TAP_REPO" --head "$BRANCH" --state open --json url --jq '.[0].url // empty')
  if [[ -n "$existing_pr" ]]; then
    echo "Open PR already exists: $existing_pr"
    exit 0
  fi
  die "remote branch already exists with no open PR: ${BRANCH}. Re-run with --branch <new-branch>."
fi

git add "$FORMULA_PATH"
git commit --quiet -m "chore: update recall to ${VERSION}"
git push --quiet -u origin "$BRANCH"

PR_BODY="$WORKDIR/pr-body.md"
cat > "$PR_BODY" <<EOF
## Summary
- update Recall formula from ${OLD_VERSION:-unknown} to ${VERSION}
- refresh SHA256 checksums for macOS arm64, macOS x86_64, and Linux x86_64 release assets

## Verification
- verified GitHub release exists: ${SOURCE_REPO}@${TAG}
- downloaded release assets and computed SHA256 checksums
- ran \`ruby -c ${FORMULA_PATH}\`
EOF

PR_URL=$(gh pr create \
  --repo "$TAP_REPO" \
  --base "$BASE_BRANCH" \
  --head "$BRANCH" \
  --title "chore: update recall to ${VERSION}" \
  --body-file "$PR_BODY")

echo "Created PR: $PR_URL"
PR_NUMBER="${PR_URL##*/}"

if [[ "$WATCH" -eq 1 ]]; then
  gh pr checks "$PR_NUMBER" --repo "$TAP_REPO" --watch --interval 10
fi

gh pr view "$PR_NUMBER" --repo "$TAP_REPO" --json url,state,mergedAt,statusCheckRollup --jq '{url,state,mergedAt,checks:[.statusCheckRollup[]? | {name:.name,status:.status,conclusion:.conclusion}]}'
