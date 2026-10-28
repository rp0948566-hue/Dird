#!/usr/bin/env python3
"""Build the canonical RetinaCare vault layout from the backend, GUI and old Dird/memory vaults.

  consolidate-vaults.py build <staging_dir>   copy + re-link notes into a NEW directory (never writes in place)
  consolidate-vaults.py check <vault_dir>     links resolve? sources changed since import? (read-only)

Rules: nothing is overwritten (build refuses a non-empty staging dir); every wikilink is rewritten to a
vault-root path so duplicate basenames cannot be ambiguous; every note keeps a provenance comment and its
source hash is recorded in Archive/migration-manifest.json so a later `check` reveals sources that changed
after import instead of letting a stale copy silently replace a newer note.
"""
from __future__ import annotations

import hashlib
import json
import os
import re
import sys
from pathlib import Path

DESKTOP = Path("/home/claude/Desktop")
BACKEND_SRC = DESKTOP / "Hackthon dezine/Hackthon Backend"          # newest backend notes
GUI_SRC = DESKTOP / "Dird/GUI/Hackthon dezine/memory"               # authoritative GUI vault (relocated copy)
STALE_SRC = DESKTOP / "Dird/memory"                                 # older backend copy inside Dird
OLD_GUI_REPO = DESKTOP / "Hackthon dezine"
NEW_GUI_REPO = DESKTOP / "Dird/GUI/Hackthon dezine"
OLD_LOCATIONS = {  # where each source's relative links were originally written from
    "backend": BACKEND_SRC,
    "gui": OLD_GUI_REPO / "memory",
    "stale": STALE_SRC,
}
TODAY = "2026-10-28"

BACKEND_MAP = {
    "architecture": "Architecture/architecture", "architecture-overview": "Architecture/architecture-overview",
    "source-map": "Architecture/source-map",
    "licensing-and-data": "Models/licensing-and-data", "model-inventory": "Models/model-inventory",
    "model-pipeline": "Models/model-pipeline", "stage-i-screening": "Models/stage-i-screening",
    "stage-ii-segmentation": "Models/stage-ii-segmentation", "stage-iii-grading": "Models/stage-iii-grading",
    "build-and-test": "Testing/build-and-test", "cuda-and-packaging": "Testing/cuda-and-packaging",
    "decisions": "Decisions/decisions", "gotchas": "Decisions/gotchas", "risks-and-blockers": "Decisions/risks-and-blockers",
    "90-claude-session-template": "Handoffs/90-claude-session-template", "agent-ownership": "Handoffs/agent-ownership",
    "changelog-and-handoffs": "Handoffs/changelog-and-handoffs", "working-agreement": "Handoffs/working-agreement",
    "data-contracts": "Backend/data-contracts", "project-overview": "Backend/project-overview", "security": "Backend/security",
    "gui-implementation": "Integration/gui-source-findings",
}
BACKEND_LINK_REDIRECT = {"00-index": "00-index", "01-claude-start-here": "01-claude-start-here",
                         "active-phase": "active-phase", "next-actions": "Handoffs/next-actions"}
SUPERSEDED = ["00-index", "01-claude-start-here", "active-phase", "next-actions", "README", "docs-index",
              "graph-overview", "retinacare-current-state", "retinacare-phase-1", "eye-ok"]
GUI_RENAMED = {"00-index": "gui-00-index", "01-claude-start-here": "gui-01-claude-start-here",
               "active-phase": "gui-active-phase", "retinacare-current-state": "gui-current-state"}
GUI_MIRROR_DIRS = {"Architecture", "Backend", "Decisions", "Handoffs", "Models", "Testing"}
MIRROR_REDIRECT = {"Handoffs/backend-active-phase": "active-phase", "Handoffs/backend-index": "00-index",
                   "Handoffs/backend-start-here": "01-claude-start-here", "Handoffs/backend-vault-readme": "00-index",
                   "Handoffs/retinacare-current-state": "active-phase"}

WIKI = re.compile(r"\[\[([^\]|#]+)(#[^\]|]*)?(\|[^\]]*)?\]\]")
MDLINK = re.compile(r"\]\((<[^>]+>|[^)\s]+)\)")
FM_REF = re.compile(r'"(\.\./[^"]+)"')


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def notes(root: Path) -> list[str]:
    return sorted(str(p.relative_to(root))[:-3] for p in root.rglob("*.md") if ".obsidian" not in p.parts)


def file_dest_of(source: str, rel: str) -> str:
    """Where the source note's own file is stored (redirected mirrors are archived, not dropped)."""
    if source == "gui" and rel in MIRROR_REDIRECT:
        return f"Archive/gui-vault-backend-mirrors/{rel}"
    if source == "backend" and rel in BACKEND_LINK_REDIRECT:
        return f"Archive/hackthon-backend-superseded/{rel}"
    return dest_of(source, rel)


def dest_of(source: str, rel: str) -> str:
    """Link target (no .md) for a source note; redirected mirrors point at the canonical hand-written note."""
    if source == "backend":
        if rel in BACKEND_LINK_REDIRECT:
            return BACKEND_LINK_REDIRECT[rel]
        if rel in BACKEND_MAP:
            return BACKEND_MAP[rel]
        if rel in SUPERSEDED:
            return f"Archive/hackthon-backend-superseded/{rel}"
        raise SystemExit(f"unmapped backend note: {rel}")
    if source == "stale":
        return f"Archive/dird-memory-pre-consolidation/{rel}"
    top = rel.split("/")[0]
    if "/" not in rel:
        return "GUI/" + GUI_RENAMED.get(rel, rel)
    if top == "Frontend":
        return "GUI/" + rel
    if top in GUI_MIRROR_DIRS:
        if rel in MIRROR_REDIRECT:
            return MIRROR_REDIRECT[rel]
        return f"Archive/gui-vault-backend-mirrors/{rel}"
    raise SystemExit(f"unmapped GUI note: {rel}")


def resolver(source: str, index: list[str]):
    def resolve(target: str, from_rel: str) -> str | None:
        t = target.strip().removesuffix(".md")
        if t in index:
            return t
        base = t.split("/")[-1]
        cand = [p for p in index if p == t or p.endswith("/" + t) or ("/" not in t and p.split("/")[-1] == base)]
        if not cand:
            cand = [p for p in index if p.split("/")[-1].lower() == base.lower()]
        if not cand:
            return None
        same_dir = [p for p in cand if os.path.dirname(p) == os.path.dirname(from_rel)]
        return sorted(same_dir or cand, key=lambda p: (p.count("/"), p))[0]
    return resolve


def map_fs_target(abs_target: Path) -> Path:
    """Old GUI repo files now live in the relocated copy."""
    try:
        return NEW_GUI_REPO / abs_target.relative_to(OLD_GUI_REPO)
    except ValueError:
        return abs_target


def rewrite(source: str, rel: str, text: str, resolve, dest_rel: str, unresolved: list) -> str:
    old_dir = (OLD_LOCATIONS[source] / rel).parent
    new_dir = Path("/virtual-vault") / dest_rel
    new_dir = new_dir.parent

    def vault_note(abs_path: Path):
        try:
            return str(abs_path.relative_to(OLD_LOCATIONS[source]))
        except ValueError:
            return None

    def wiki(m):
        target, anchor, alias = m.group(1), m.group(2) or "", m.group(3) or ""
        hit = resolve(target, rel)
        if hit is None:
            unresolved.append((dest_rel, "wikilink", target))
            return m.group(0)
        return f"[[{dest_of(source, hit)}{anchor}{alias}]]"

    def relative(ref: str) -> str | None:
        clean = ref.split("#")[0]
        anchor = ref[len(clean):]
        abs_old = Path(os.path.normpath(old_dir / clean))
        note = vault_note(abs_old)
        if note is not None and note.endswith(".md"):
            hit = resolve(note[:-3], rel)
            if hit:
                new_abs = Path("/virtual-vault") / (dest_of(source, hit) + ".md")
                return os.path.relpath(new_abs, new_dir) + anchor
        target = map_fs_target(abs_old)
        if target.exists():
            if str(target).startswith("/home/claude/Desktop/Dird/memory"):
                return None
            # Repo files are addressed from the final vault location /home/claude/Desktop/Dird/memory/<dest_rel>.
            final_dir = Path("/home/claude/Desktop/Dird/memory") / os.path.dirname(dest_rel)
            return os.path.relpath(target, final_dir) + anchor
        unresolved.append((dest_rel, "path", ref))
        return None

    def md(m):
        raw = m.group(1)
        inner = raw[1:-1] if raw.startswith("<") else raw
        if re.match(r"^(https?:|mailto:|#)", inner):
            return m.group(0)
        new = relative(inner)
        if new is None:
            return m.group(0)
        return f"]({'<' + new + '>' if raw.startswith('<') else new})"

    head, sep, rest = "", "", text
    if text.startswith("---\n"):
        end = text.find("\n---\n", 4)
        if end != -1:
            head, rest = text[: end + 5], text[end + 5:]

    def fm(m):
        new = relative(m.group(1))
        return f'"{new}"' if new else m.group(0)

    head = FM_REF.sub(fm, head)
    rest = WIKI.sub(wiki, rest)
    rest = MDLINK.sub(md, rest)
    return head + rest


def build(staging: Path) -> None:
    if staging.exists() and any(staging.iterdir()):
        raise SystemExit(f"refusing to write into non-empty {staging}")
    staging.mkdir(parents=True, exist_ok=True)
    manifest, unresolved = [], []
    for source, root in (("backend", BACKEND_SRC), ("gui", GUI_SRC), ("stale", STALE_SRC)):
        index = notes(root)
        resolve = resolver(source, index)
        for rel in index:
            src = root / (rel + ".md")
            dest_rel = file_dest_of(source, rel)
            out = staging / (dest_rel + ".md")
            if out.exists():
                raise SystemExit(f"destination collision: {dest_rel} (from {source}:{rel})")
            out.parent.mkdir(parents=True, exist_ok=True)
            text = rewrite(source, rel, src.read_text(), resolve, dest_rel, unresolved)
            note = f"<!-- migrated {TODAY} from {src} (sha256 {sha(src)[:12]}); verify current state in [[active-phase]] -->\n"
            if text.startswith("---\n") and "\n---\n" in text[4:]:
                end = text.find("\n---\n", 4) + 5
                text = text[:end] + note + text[end:]
            else:
                text = note + text
            out.write_text(text)
            manifest.append({"source": source, "src": str(src), "src_sha256": sha(src),
                             "dest": dest_rel + ".md", "dest_sha256": sha(out)})
    (staging / "Archive").mkdir(exist_ok=True)
    (staging / "Archive/migration-manifest.json").write_text(json.dumps(
        {"migrated": TODAY, "notes": manifest, "unresolved": [list(u) for u in unresolved]}, indent=1) + "\n")
    print(f"built {len(manifest)} notes into {staging}; unresolved references: {len(unresolved)}")
    for item in unresolved[:30]:
        print("  unresolved", item)


def check(vault: Path) -> int:
    index = set(notes(vault))
    broken, total = [], 0
    for rel in sorted(index):
        text = (vault / (rel + ".md")).read_text()
        for m in WIKI.finditer(text):
            total += 1
            t = m.group(1).strip().removesuffix(".md")
            if t not in index:
                broken.append((rel, t))
    print(f"wikilinks: {total} checked, {len(broken)} unresolved")
    for item in broken[:40]:
        print("  broken", item)
    bad_rel = []
    for rel in sorted(index):
        text = (vault / (rel + ".md")).read_text()
        for m in MDLINK.finditer(text):
            raw = m.group(1)
            inner = raw[1:-1] if raw.startswith("<") else raw
            if re.match(r"^(https?:|mailto:|#)", inner):
                continue
            path = inner.split("#")[0]
            if path and not (vault / rel).parent.joinpath(path).resolve().exists():
                bad_rel.append((rel, inner))
    print(f"relative links: {len(bad_rel)} unresolved")
    for item in bad_rel[:40]:
        print("  missing", item)
    manifest = vault / "Archive/migration-manifest.json"
    drifted = []
    if manifest.exists():
        for entry in json.loads(manifest.read_text())["notes"]:
            src = Path(entry["src"])
            if src.exists() and sha(src) != entry["src_sha256"]:
                drifted.append(entry["src"])
    print(f"sources changed since import: {len(drifted)}")
    for item in drifted:
        print("  CHANGED-SOURCE", item)
    return 1 if broken else 0


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "build":
        build(Path(sys.argv[2]))
    elif len(sys.argv) == 3 and sys.argv[1] == "check":
        raise SystemExit(check(Path(sys.argv[2])))
    else:
        raise SystemExit(__doc__)
