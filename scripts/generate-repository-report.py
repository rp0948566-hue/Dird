#!/usr/bin/env python3
"""Generate a measured RetinaCare/DIRD+ repository report."""

from __future__ import annotations

from collections import Counter
from datetime import datetime
from pathlib import Path
import json
import subprocess
import tomllib
from xml.sax.saxutils import escape

from reportlab.graphics.charts.barcharts import VerticalBarChart
from reportlab.graphics.charts.piecharts import Pie
from reportlab.graphics.shapes import Drawing, Line, Polygon, Rect, String
from reportlab.lib import colors
from reportlab.lib.enums import TA_CENTER, TA_LEFT
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import mm
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.platypus import (
    Image,
    KeepTogether,
    PageBreak,
    Paragraph,
    SimpleDocTemplate,
    Spacer,
    Table,
    TableStyle,
)

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "docs/RetinaCare-Repository-Technical-Report.pdf"
EXCLUDED = ("dist/", "native/target/", "node_modules/", ".git/", ".serena/")
BLUE = colors.HexColor("#276FB9")
DARK = colors.HexColor("#202832")
INK = colors.HexColor("#26323D")
MUTED = colors.HexColor("#667482")
PALE = colors.HexColor("#EEF4F8")
GREEN = colors.HexColor("#2E7D66")
ORANGE = colors.HexColor("#C47A24")


def run(*args: str) -> str:
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def register_fonts() -> tuple[str, str, str]:
    directory = Path("/usr/share/fonts/truetype/dejavu")
    if directory.exists():
        pdfmetrics.registerFont(TTFont("RC", directory / "DejaVuSans.ttf"))
        pdfmetrics.registerFont(TTFont("RC-Bold", directory / "DejaVuSans-Bold.ttf"))
        pdfmetrics.registerFont(TTFont("RC-Mono", directory / "DejaVuSansMono.ttf"))
        return "RC", "RC-Bold", "RC-Mono"
    return "Helvetica", "Helvetica-Bold", "Courier"


FONT, BOLD, MONO = register_fonts()


def source_files() -> list[Path]:
    paths = run("git", "ls-files", "--cached", "--others", "--exclude-standard").splitlines()
    return [ROOT / path for path in paths if path and not path.startswith(EXCLUDED) and "__pycache__" not in path]


LANGUAGES = {
    ".rs": "Rust", ".py": "Python", ".ts": "TypeScript", ".tsx": "TypeScript/React",
    ".js": "JavaScript", ".cjs": "JavaScript", ".json": "JSON", ".toml": "TOML",
    ".md": "Markdown", ".html": "HTML", ".css": "CSS", ".yml": "YAML", ".yaml": "YAML",
    ".ps1": "PowerShell", ".sh": "Shell", ".sql": "SQL", ".svg": "SVG", ".txt": "Text",
    ".lock": "Lockfile", ".xml": "XML", ".csv": "CSV",
}


def language(path: Path) -> str:
    if path.name in {"LICENSE", "Dockerfile"}: return "Text"
    return LANGUAGES.get(path.suffix.lower(), "Binary/other")


def area(relative: str) -> str:
    first = relative.split("/", 1)[0]
    return {
        "native": "Native RetinaCare", "ai_engine": "AI worker", "src": "Legacy web UI",
        "src-tauri": "Legacy Tauri", "validation": "Model validation", "docs": "Documentation",
        "public": "Legacy assets", "scripts": "Build tooling", ".github": "CI/CD",
        "ponytail": "Engineering rules", "memory": "Project memory", "models": "Model contract",
        "icon": "Brand assets",
    }.get(first, "Repository root")


def requirement(relative: str) -> str:
    if relative.startswith(("native/", "ai_engine/", "models/", "icon/")): return "Native runtime/source"
    if relative.startswith(("scripts/", ".github/")): return "Build and verification"
    if relative.startswith(("src/", "src-tauri/", "public/")): return "Preserved legacy reference"
    if relative.startswith("validation/"): return "Scientific evidence"
    if relative.startswith(("docs/", "memory/", "ponytail/")): return "Documentation/tooling"
    return "Repository configuration"


def purpose(relative: str) -> str:
    name = Path(relative).name
    if name == "Cargo.toml": return "Rust package/workspace manifest"
    if name == "Cargo.lock": return "Locked Rust dependency graph"
    if name == "package.json": return "Legacy JavaScript commands and direct dependencies"
    if name == "pyproject.toml": return "Python AI package metadata"
    if name.endswith(".test.ts") or "tests/" in relative: return "Automated regression test"
    if name == "main.rs": return "Native desktop entry point and interactive UI"
    if name == "worker.py": return "Persistent local AI IPC worker"
    if name == "database.rs": return "Nonblocking encrypted database controller"
    if name == "resources.rs": return "Portable runtime and user-data path discovery"
    if name == "build.rs": return "Windows executable resource embedding"
    if relative.startswith("native/"): return "Native Rust application component"
    if relative.startswith("ai_engine/"): return "Python AI worker component"
    if relative.startswith("src-tauri/"): return "Preserved DIRD+ Rust backend/reference"
    if relative.startswith("src/"): return "Preserved DIRD+ web UI/reference"
    if relative.startswith("validation/"): return "Existing model experiment or validation evidence"
    if relative.startswith("docs/"): return "Project documentation or technical evidence"
    if relative.startswith("scripts/"): return "Build, packaging, test, or report automation"
    if relative.startswith(".github/"): return "Continuous integration/release workflow"
    if relative.startswith("ponytail/"): return "Imported repository engineering workflow rule"
    if relative.startswith("memory/"): return "Obsidian-compatible project memory"
    if relative.startswith("public/"): return "Legacy static application asset"
    if relative.startswith("models/"): return "Model placement or manifest contract"
    if relative.startswith("icon/"): return "RetinaCare icon source or executable resource"
    return "Repository configuration, policy, or project information"


def line_count(path: Path) -> int | None:
    if language(path) == "Binary/other" or path.stat().st_size > 4_000_000: return None
    try: return len(path.read_text(encoding="utf-8", errors="strict").splitlines())
    except (UnicodeDecodeError, OSError): return None


def direct_dependencies() -> list[tuple[str, str, str]]:
    rows: list[tuple[str, str, str]] = []
    workspace = tomllib.loads((ROOT / "native/Cargo.toml").read_text())
    for crate in workspace["workspace"]["members"]:
        manifest = tomllib.loads((ROOT / "native" / crate / "Cargo.toml").read_text())
        for name, value in manifest.get("dependencies", {}).items():
            version = value if isinstance(value, str) else value.get("version", "workspace/local")
            rows.append(("Rust", name, str(version)))
    package = json.loads((ROOT / "package.json").read_text())
    for group in ("dependencies", "devDependencies"):
        for name, version in package.get(group, {}).items(): rows.append(("Legacy JS", name, version))
    python = tomllib.loads((ROOT / "ai_engine/pyproject.toml").read_text())
    for item in python.get("project", {}).get("dependencies", []): rows.append(("Python", item, "declared"))
    return sorted(set(rows))


def architecture_diagram() -> Drawing:
    drawing = Drawing(500, 230)
    boxes = [
        (15, 155, 105, 48, "Native UI\neframe / egui", BLUE),
        (145, 155, 105, 48, "Application\ncontrollers", GREEN),
        (280, 155, 95, 48, "Domain +\nreports", colors.HexColor("#6B5FA7")),
        (400, 155, 85, 48, "SQLCipher\nSQLite", ORANGE),
        (145, 45, 105, 48, "AI bridge\nJSONL IPC", colors.HexColor("#2D7C8D")),
        (290, 45, 105, 48, "Python worker\nPyTorch probe", colors.HexColor("#8B5A2B")),
        (415, 45, 70, 48, "CUDA /\nmodels", colors.HexColor("#587A42")),
    ]
    for x, y, w, h, label, fill in boxes:
        drawing.add(Rect(x, y, w, h, 6, 6, fillColor=fill, strokeColor=None))
        for index, line in enumerate(label.splitlines()):
            drawing.add(String(x + w / 2, y + h / 2 + 7 - index * 14, line, textAnchor="middle", fillColor=colors.white, fontName=BOLD, fontSize=9))
    for x1, y1, x2, y2 in [(120,179,145,179),(250,179,280,179),(375,179,400,179),(197,155,197,93),(250,69,290,69),(395,69,415,69)]:
        drawing.add(Line(x1, y1, x2, y2, strokeColor=MUTED, strokeWidth=1.5))
        drawing.add(Polygon([x2, y2, x2-6, y2+3, x2-6, y2-3], fillColor=MUTED, strokeColor=None))
    drawing.add(String(250, 215, "RetinaCare target architecture", textAnchor="middle", fillColor=DARK, fontName=BOLD, fontSize=13))
    return drawing


def pipeline_diagram() -> Drawing:
    drawing = Drawing(500, 140)
    labels = ["Select patient", "Create exam", "Import image", "Validate image", "Worker IPC", "Save encrypted", "Reopen report"]
    width = 62
    for index, label in enumerate(labels):
        x = 4 + index * 71
        drawing.add(Rect(x, 48, width, 45, 5, 5, fillColor=PALE, strokeColor=BLUE))
        words = label.split()
        for line, word in enumerate(words): drawing.add(String(x + width / 2, 74-line*11, word, textAnchor="middle", fontName=FONT, fontSize=7.5, fillColor=INK))
        if index < len(labels) - 1:
            drawing.add(Line(x + width, 70, x + 70, 70, strokeColor=BLUE))
            drawing.add(Polygon([x+70,70,x+65,73,x+65,67], fillColor=BLUE, strokeColor=None))
    drawing.add(String(250, 115, "Implemented local workflow", textAnchor="middle", fillColor=DARK, fontName=BOLD, fontSize=12))
    drawing.add(String(250, 20, "AI stage returns a truthful missing-model error until verified weights and an adapter exist.", textAnchor="middle", fillColor=MUTED, fontName=FONT, fontSize=8))
    return drawing


def bar_chart(counts: Counter[str]) -> Drawing:
    top = counts.most_common(10)
    drawing = Drawing(500, 220)
    chart = VerticalBarChart()
    chart.x, chart.y, chart.width, chart.height = 45, 45, 430, 135
    chart.data = [[value for _, value in top]]
    chart.categoryAxis.categoryNames = [name.replace(" ", "\n") for name, _ in top]
    chart.categoryAxis.labels.fontName = FONT
    chart.categoryAxis.labels.fontSize = 6
    chart.valueAxis.labels.fontName = FONT
    chart.valueAxis.labels.fontSize = 7
    chart.bars[0].fillColor = BLUE
    drawing.add(chart)
    drawing.add(String(250, 202, "Repository files by area", textAnchor="middle", fontName=BOLD, fontSize=12, fillColor=DARK))
    return drawing


def pie_chart(lines: Counter[str]) -> Drawing:
    top = lines.most_common(8)
    other = sum(lines.values()) - sum(value for _, value in top)
    if other: top.append(("Other", other))
    drawing = Drawing(500, 220)
    pie = Pie()
    pie.x, pie.y, pie.width, pie.height = 35, 25, 165, 165
    pie.data = [value for _, value in top]
    pie.labels = None
    pie.slices.strokeWidth = 0.5
    for index, color in enumerate([BLUE, GREEN, ORANGE, colors.HexColor("#6B5FA7"), colors.HexColor("#2D7C8D"), colors.HexColor("#9C6B76"), colors.HexColor("#72804E"), colors.HexColor("#8C7A62"), colors.lightgrey]):
        pie.slices[index].fillColor = color
    drawing.add(pie)
    total = sum(value for _, value in top)
    for index, (name, value) in enumerate(top):
        y = 172 - index * 17
        drawing.add(Rect(235, y - 5, 9, 9, fillColor=pie.slices[index].fillColor, strokeColor=None))
        drawing.add(String(250, y, f"{name}: {value:,} lines ({value / total:.1%})", fontName=FONT, fontSize=8, fillColor=INK))
    drawing.add(String(250, 207, "Measured text lines by language", textAnchor="middle", fontName=BOLD, fontSize=12, fillColor=DARK))
    return drawing


def make_styles():
    styles = getSampleStyleSheet()
    return {
        "title": ParagraphStyle("title", parent=styles["Title"], fontName=BOLD, fontSize=28, leading=33, textColor=DARK, alignment=TA_LEFT, spaceAfter=12),
        "subtitle": ParagraphStyle("subtitle", parent=styles["Normal"], fontName=FONT, fontSize=11, leading=16, textColor=MUTED),
        "h1": ParagraphStyle("h1", parent=styles["Heading1"], fontName=BOLD, fontSize=18, leading=23, textColor=DARK, spaceBefore=12, spaceAfter=8),
        "h2": ParagraphStyle("h2", parent=styles["Heading2"], fontName=BOLD, fontSize=13, leading=17, textColor=BLUE, spaceBefore=10, spaceAfter=5),
        "body": ParagraphStyle("body", parent=styles["BodyText"], fontName=FONT, fontSize=8.8, leading=13, textColor=INK, spaceAfter=6),
        "small": ParagraphStyle("small", parent=styles["BodyText"], fontName=FONT, fontSize=6.6, leading=9, textColor=INK),
        "code": ParagraphStyle("code", parent=styles["Code"], fontName=MONO, fontSize=7.2, leading=10, backColor=PALE, borderPadding=6, spaceAfter=6),
        "center": ParagraphStyle("center", parent=styles["BodyText"], fontName=FONT, fontSize=8, alignment=TA_CENTER, textColor=MUTED),
    }


def table(rows, widths, header=True, font_size=7):
    formatted = [[Paragraph(escape(str(cell)), STYLES["small"]) for cell in row] for row in rows]
    result = Table(formatted, colWidths=widths, repeatRows=1 if header else 0, hAlign="LEFT")
    commands = [
        ("VALIGN", (0,0), (-1,-1), "TOP"), ("GRID", (0,0), (-1,-1), 0.25, colors.HexColor("#CBD5DD")),
        ("LEFTPADDING", (0,0), (-1,-1), 4), ("RIGHTPADDING", (0,0), (-1,-1), 4),
        ("TOPPADDING", (0,0), (-1,-1), 3), ("BOTTOMPADDING", (0,0), (-1,-1), 3),
        ("FONTSIZE", (0,0), (-1,-1), font_size),
    ]
    if header:
        commands += [("BACKGROUND", (0,0), (-1,0), DARK), ("TEXTCOLOR", (0,0), (-1,0), colors.white), ("FONTNAME", (0,0), (-1,0), BOLD)]
    for row in range(1 if header else 0, len(rows)):
        if row % 2 == 0: commands.append(("BACKGROUND", (0,row), (-1,row), colors.HexColor("#F6F8FA")))
    result.setStyle(TableStyle(commands))
    return result


def footer(canvas, doc):
    canvas.saveState()
    canvas.setStrokeColor(colors.HexColor("#D8E0E6")); canvas.line(18*mm, 14*mm, 192*mm, 14*mm)
    canvas.setFont(FONT, 7); canvas.setFillColor(MUTED)
    canvas.drawString(18*mm, 9*mm, "RetinaCare repository technical report • generated from local working tree")
    canvas.drawRightString(192*mm, 9*mm, f"Page {doc.page}")
    canvas.restoreState()


FILES = source_files()
TRACKED = set(run("git", "ls-files").splitlines())
MODIFIED = set(run("git", "diff", "--name-only").splitlines())
records = []
language_lines: Counter[str] = Counter()
area_counts: Counter[str] = Counter()
for file in FILES:
    relative = file.relative_to(ROOT).as_posix()
    lines = line_count(file)
    lang = language(file)
    if lines is not None: language_lines[lang] += lines
    area_counts[area(relative)] += 1
    state = "modified" if relative in MODIFIED else ("tracked" if relative in TRACKED else "new/untracked")
    records.append((relative, area(relative), requirement(relative), lang, lines, file.stat().st_size, state, purpose(relative)))

STYLES = make_styles()
story = []
icon = ROOT / "icon/9056c431-3552-45dc-ba90-ef8a0373ee3a-cleaned.png"
if icon.exists(): story += [Spacer(1, 8*mm), Image(str(icon), width=29*mm, height=29*mm)]
story += [Paragraph("RetinaCare / DIRD+", STYLES["title"]), Paragraph("Complete repository, architecture, technology, pipeline, file inventory, work attribution, build and verification report", STYLES["subtitle"]), Spacer(1, 8*mm)]
commit = run("git", "rev-parse", "--short", "HEAD")
story += [table([
    ["Report date", datetime.now().astimezone().strftime("%Y-%m-%d %H:%M %Z")],
    ["Repository", str(ROOT)], ["Baseline commit", commit], ["Inventoried files", f"{len(records):,}"],
    ["Measured text lines", f"{sum(language_lines.values()):,}"], ["Working tree", "Contains local RetinaCare migration changes; not committed"],
], [45*mm, 125*mm], header=False, font_size=8), Spacer(1, 8*mm), Paragraph("Evidence boundary", STYLES["h2"]), Paragraph("This report distinguishes verified implementation from unverified claims. No Windows execution, RTX 5070/CUDA benchmark, trained-model diagnosis, model accuracy, heatmap, or clinical outcome is asserted.", STYLES["body"]), PageBreak()]

story += [Paragraph("1. Executive repository overview", STYLES["h1"]), Paragraph(f"The working tree contains {len(records):,} source, configuration, documentation, validation, asset, and engineering-rule files after excluding generated builds, dependencies, caches, and Git internals. DIRD+ remains preserved as the legacy reference while RetinaCare is implemented beside it under native/ and ai_engine/.", STYLES["body"]), bar_chart(area_counts), PageBreak(), Paragraph("1.1 Language profile", STYLES["h1"]), pie_chart(language_lines), Paragraph("Counts are physical text lines, not semantic lines of code. Generated dependencies and build output are excluded; checked-in validation CSV data and documentation are included and therefore visible in the distribution.", STYLES["body"]), PageBreak()]

story += [Paragraph("2. Architecture and runtime data flow", STYLES["h1"]), architecture_diagram(), Paragraph("The native UI never exposes generic SQL and does not use browser storage. Rust owns application orchestration and SQLCipher access. A persistent local Python subprocess exchanges bounded, versioned JSON Lines over stdin/stdout. No HTTP server or WebView exists in the new runtime.", STYLES["body"]), pipeline_diagram(), PageBreak()]

story += [Paragraph("3. What has been completed", STYLES["h1"]), table([
    ["Area", "Implemented and verified state"],
    ["Native UI", "eframe/egui entry point, icon, dark navigation, dashboard, patient registry, screening viewer, AI results, history, reports, settings, GPU/model status"],
    ["Patients", "Open/create encrypted database, add, edit, search, select, open records"],
    ["Screening", "Select patient/eye, native file picker, bounded background image decode, zoom, pan, reset, analyze request, encrypted save"],
    ["AI worker", "Persistent protocol, health/device probe, model discovery, preprocessing validation, timing, explicit missing-model errors"],
    ["Persistence", "SQLCipher, Argon2id, patient/examination records, image/eye/analysis fields, safe rekey recovery"],
    ["Reports", "Reopen saved examinations and real analysis payload; JSON export. PDF clinical report export is not implemented"],
    ["Packaging", "Ubuntu portable package and real Windows PE portable package with pinned Python runtime"],
    ["Safety", "No fabricated findings; bounded IPC/images; zeroized credentials; atomic legacy import; no native generic SQL"],
], [38*mm, 132*mm]), Spacer(1, 6*mm), Paragraph("Current verification", STYLES["h2"]), table([
    ["Gate", "Recorded result"], ["Legacy frontend", "59 tests + type-check + production build passed in Phase 1"],
    ["Native Rust", "12 tests, formatting, Clippy and release build passed after GUI workflow expansion"],
    ["Python worker", "5 tests passed"], ["Ubuntu package", "Headless worker/database/icon smoke passed"],
    ["Ubuntu GUI", "Real packaged Wayland launch captured; automated 8-screen lifecycle/worker smoke passed"],
    ["Windows PE", "Cross-build, GUI subsystem and embedded resources verified; latest GUI rebuild pending"],
    ["Security", "RustSec: no vulnerabilities; three unmaintained Linux accessibility transitives"],
], [45*mm, 125*mm]), PageBreak()]
running = ROOT / "docs/assets/retinacare-ubuntu-running.png"
if running.exists():
    story += [Paragraph("3.1 Verified Ubuntu GUI evidence", STYLES["h1"]), Paragraph("Captured by egui's viewport screenshot callback from the packaged native application on the active Ubuntu Wayland desktop. Worker health is OK and missing model weights are shown truthfully.", STYLES["body"]), Image(str(running), width=174*mm, height=108.75*mm), Paragraph("Automated GUI report: 71 frames, eight navigation screens, worker health observed, passed.", STYLES["center"]), PageBreak()]

story += [Paragraph("4. Technologies and libraries", STYLES["h1"]), table([
    ["Layer", "Technology", "Purpose"], ["Desktop UI", "Rust, eframe, egui", "Native cross-platform windowing and immediate-mode UI"],
    ["Application", "Rust std threads/channels", "Bounded nonblocking UI controllers and cleanup"],
    ["Database", "SQLite + SQLCipher via rusqlite", "Encrypted local patient/examination storage"],
    ["Key derivation", "Argon2id + OS randomness", "Password-derived 256-bit database key"],
    ["AI process", "Python 3.13", "Persistent local structured worker"], ["Imaging", "Pillow + Rust image", "Input validation and native display"],
    ["GPU", "Optional PyTorch/CUDA", "Detection only today; trained adapter still required"],
    ["IPC", "Versioned JSON Lines", "Bounded stdin/stdout messages; no network server"],
    ["Legacy", "React/TypeScript/Tauri", "Preserved source for feature migration, not final native runtime"],
    ["Build", "Cargo, PowerShell, Bash, GitHub Actions", "Tests and portable Ubuntu/Windows artifacts"],
], [30*mm, 47*mm, 93*mm]), Spacer(1, 7*mm)]
deps = direct_dependencies()
story += [Paragraph(f"Direct dependency declarations ({len(deps)})", STYLES["h2"]), table([["Ecosystem", "Dependency", "Declared version"]] + deps, [28*mm, 92*mm, 50*mm]), PageBreak()]

story += [Paragraph("5. Work attribution and provenance", STYLES["h1"]), Paragraph("Git commit attribution describes the preserved DIRD+ history. It does not attribute the current uncommitted RetinaCare migration line-by-line.", STYLES["body"])]
authors = []
for row in run("git", "shortlog", "-sne", "HEAD").splitlines():
    count, identity = row.strip().split("\t", 1); authors.append([count, identity])
story += [table([["Commits", "Git author identity"]] + authors, [25*mm, 145*mm]), Spacer(1, 6*mm), Paragraph("Current local work", STYLES["h2"]), Paragraph("The Phase 1/1.5 RetinaCare changes are present only in this local working tree. They were produced through Codex-assisted implementation under the user's direction. No commit author, pull request, or remote publication exists yet, so the report does not invent individual authorship for those changes.", STYLES["body"]), Paragraph("Licensing", STYLES["h2"]), Paragraph("The repository retains AGPL-3.0 licensing for DIRD+ and reused code. The native migration must preserve notices. Dataset licenses, model licenses, and patient-data approvals remain separate mandatory checks.", STYLES["body"]), PageBreak()]

story += [Paragraph("6. Security, privacy, and clinical boundary", STYLES["h1"]), table([
    ["Control", "Current implementation"], ["Patient data at rest", "SQLCipher database under per-user application data"],
    ["Passwords", "Argon2id-derived key; Rust buffers zeroized where practical; password not persisted"],
    ["Password rotation", "Staged salt plus recovery candidates after rekey"], ["Imports", "Legacy .dird writes wrapped in atomic Dexie transaction"],
    ["IPC", "Local child process only; request/response sizes, IDs, protocol, status, timeout, and reap enforced"],
    ["Images", "Regular-file, 100 MiB, 40 MP and allocation limits; decode off UI thread"],
    ["Model output", "No diagnosis or findings unless a verified adapter and weights actually return them"],
    ["Logs", "Worker stderr stored locally; never print passwords or patient image bytes"],
], [42*mm, 128*mm]), Spacer(1, 6*mm), Paragraph("Known gaps", STYLES["h2"]), Paragraph("No trained inference adapter, no validated clinical PDF exporter, no signed Windows installer, no Windows execution evidence, no RTX 5070 validation, and no clinical deployment approval. Existing validation assets describe DIRD+ experiments and must not be treated as validation of the new native runtime.", STYLES["body"]), PageBreak()]

story += [Paragraph("7. Build, package, and operating-system pipeline", STYLES["h1"]), Paragraph("Ubuntu development", STYLES["h2"]), Paragraph("cargo fmt → clippy → workspace tests → release build → package pinned Python/Pillow → headless worker/database/icon smoke → checksum/tar.", STYLES["code"]), Paragraph("Windows delivery", STYLES["h2"]), Paragraph("Cross-build or native Windows Cargo build → embed ICO → bundle isolated Python worker → checksum/ZIP → reboot into Windows → test-windows.ps1 → manual image/database/GPU checklist.", STYLES["code"]), table([
    ["Artifact", "Location / status"], ["Ubuntu executable", "dist/RetinaCare-Ubuntu/RetinaCare"],
    ["Ubuntu archive", "dist/RetinaCare-Ubuntu-x86_64.tar.gz"], ["Windows executable", "dist/RetinaCare/RetinaCare.exe"],
    ["Windows portable ZIP", "dist/RetinaCare-Windows-x86_64.zip"], ["Windows smoke", "scripts/test-windows.ps1"],
], [45*mm, 125*mm]), PageBreak()]

story += [Paragraph("8. Repository file inventory", STYLES["h1"]), Paragraph("Every inventoried file is listed below. Generated distributions, Cargo target output, dependency folders, caches, and Git internals are intentionally excluded because they are reproducible artifacts rather than maintained repository source.", STYLES["body"])]
inventory_rows = [["Path", "Area / need", "Lang", "Lines", "State", "Purpose"]]
for relative, item_area, need, lang, lines, size, state, item_purpose in records:
    inventory_rows.append([relative, f"{item_area}\n{need}", lang, "—" if lines is None else str(lines), state, item_purpose])
story += [table(inventory_rows, [58*mm, 35*mm, 19*mm, 12*mm, 18*mm, 42*mm], font_size=5.8), PageBreak()]

story += [Paragraph("9. Remaining work and decisions", STYLES["h1"]), table([
    ["Priority", "Required next work", "Evidence needed"],
    ["P0", "Exercise manual patient/image/report clicks with synthetic data", "Screenshots and saved/reopened encrypted examination"],
    ["P0", "Rebuild latest Windows package, boot Windows and run packaged smoke", "JSON reports, logs, screenshots, clean-exit observation"],
    ["P1", "Integrate one licensed checksum-pinned trained model adapter", "Manifest, hash, label map, preprocessing, validation evidence"],
    ["P1", "Validate RTX 5070 CUDA runtime", "Driver/Torch/CUDA versions, device name, VRAM, measured stage timings"],
    ["P1", "Implement clinical PDF reporting", "Stable report schema, clinician-review controls, golden-file tests"],
    ["P2", "Complete remaining DIRD+ feature parity", "Traceable migration matrix and regression tests"],
], [20*mm, 90*mm, 60*mm]), Spacer(1, 8*mm), Paragraph("Conclusion", STYLES["h2"]), Paragraph("RetinaCare is a real native Rust/Python/SQLCipher migration with working foundations and portable builds, while DIRD+ remains available for reference. The next honest milestone is visual GUI evidence plus Windows/RTX execution, followed by a verified trained-model adapter. Until then, the software must remain research decision support and must not produce or imply clinical diagnoses.", STYLES["body"])]

doc = SimpleDocTemplate(str(OUTPUT), pagesize=A4, rightMargin=16*mm, leftMargin=16*mm, topMargin=16*mm, bottomMargin=18*mm, title="RetinaCare Repository Technical Report", author="RetinaCare project / Codex-assisted repository analysis", subject="Repository inventory, architecture, technologies, pipelines, implementation and verification")
doc.build(story, onFirstPage=footer, onLaterPages=footer)
print(OUTPUT)
