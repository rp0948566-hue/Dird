#!/usr/bin/env bash
# Cheap whole-repo audit: run everything, read only what it flags. No AI tokens spent on clean code.
# Usage: scripts/audit.sh   (writes docs/audit-latest.md)
set -u
cd "$(dirname "$0")/.."
GUI="GUI/Hackthon dezine/rust"
out=docs/audit-latest.md
{
echo "# Audit $(date -I)"
echo; echo "## clippy (GUI)"
(cd "$GUI" && cargo clippy --all-targets 2>&1 | grep -E "^(warning|error)" | sort | uniq -c | head -20)
echo; echo "## clippy (backend)"
cargo clippy --workspace --manifest-path native/Cargo.toml 2>&1 | grep -E "^(warning|error)" | sort | uniq -c | head -20
echo; echo "## cargo audit (dependency CVEs)"
if command -v cargo-audit >/dev/null; then cargo audit --file native/Cargo.lock 2>&1 | tail -15; else echo "SKIPPED: cargo install cargo-audit"; fi
echo; echo "## secrets"
if command -v gitleaks >/dev/null; then gitleaks detect --no-banner --redact 2>&1 | tail -8; else
  rg -n --glob '!target' --glob '!RetinaCare-Training-Lab' -e '(api[_-]?key|secret|passw(or)?d|token)\s*[:=]\s*"[^"]{8,}"' native ai_engine "$GUI/src" | head -10; echo "(basic rg scan; install gitleaks for full)"; fi
echo; echo "## risky patterns, production code only (files with #[cfg(test)] counted by hits above it)"
for p in 'unsafe ' 'panic!\(' 'println!|dbg!' '\.unwrap\(\)|\.expect\('; do
  echo "- $p"; rg -n --glob '*.rs' --glob '!target' -e "$p" native/crates "$GUI/src" | rg -v '/tests?/|_tests?\.rs' | head -8
done
echo; echo "## tracked files that must not be in git"
git ls-files | rg -i '\.(db|sqlite|onnx|pth|pt|safetensors|ckpt|dcm)$' | rg -v 'fixtures/' | head
echo; echo "## tests"
(cd "$GUI" && cargo test 2>&1 | rg '^test result|FAILED' | head -5)
cargo test --workspace --manifest-path native/Cargo.toml 2>&1 | rg '^test result: F|FAILED' | head -5; echo "(no FAILED lines above = pass)"
} > "$out" 2>&1
echo "wrote $out"
