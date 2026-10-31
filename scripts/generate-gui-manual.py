#!/usr/bin/env python3
"""Auto-extract a GUI manual from source (pages, buttons, animations, pipeline stages, tests). No hand-written claims."""
import re, subprocess, datetime
from pathlib import Path
R = Path("GUI/Hackthon dezine/rust/src")
def rg(p, files="*.rs"):
    out = subprocess.run(["grep","-rEn","--include",files,p,str(R)],capture_output=True,text=True).stdout
    return [l for l in out.splitlines() if l]
def rel(l): return l.replace(str(R)+"/","")
sec = []
def add(t, rows, note=""):
    sec.append((t, note, [rel(r) for r in rows]))
add("Pages and navigation", rg(r'AppPage::\w+\s*=>|PAGES\b'), "Where each page is declared and routed.")
add("Buttons and clickable labels", rg(r'(menu_item|primary_button|ghost_button|Button::new|\.button)\(.*"[A-Z][^"]{2,40}"'), "Every button label with its source line; the handler sits within a few lines of it.")
add("Animations", rg(r'animate_bool|animate_value|request_repaint_after|smootherstep|ease_'), "Every animation primitive: bool/value tweens, eased curves, timed repaints.")
add("Pipeline stages", rg(r'StageKey::|AnalysisStage::|run_stage|fn run_cascade'), "Screening, masks, grading and explainability entry points.")
add("Chat and AI", rg(r'SYSTEM_PROMPT|fn build_messages|stage_status_reply|plain_dashes|guardrail|ungrounded_numbers'), "Prompt, grounding and guardrails.")
add("PDF report", rg(r'fn (export|build|render)\w*(pdf|report)|Retinal Fundus Examination Report'), "Report generation.")
add("Storage and security", rg(r'SQLCipher|fn delete_all|zeroize|compute_sha256|ZeroizedBuffer'), "Encrypted store, hashing, wipes.")
add("Tests", rg(r'#\[test\]'), "Every unit test marker (count shows coverage).")
md = [f"# RetinaCare GUI manual (auto-extracted {datetime.date.today()})", "", "Source of truth is the code; each row is file:line:text."]
for t,n,rows in sec:
    md += ["", f"## {t} ({len(rows)} hits)", n, ""]
    md += [f"- `{r[:160]}`" for r in rows[:400]]
Path("docs/gui-manual.md").write_text("\n".join(md))
try:
    from reportlab.lib.pagesizes import A4
    from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer, Preformatted
    from reportlab.lib.styles import getSampleStyleSheet
    from xml.sax.saxutils import escape
    st = getSampleStyleSheet(); doc = SimpleDocTemplate("docs/RetinaCare-GUI-Manual.pdf", pagesize=A4)
    f = []
    for l in md:
        if l.startswith("# "): f.append(Paragraph(escape(l[2:]), st["Title"]))
        elif l.startswith("## "): f += [Spacer(1,8), Paragraph(escape(l[3:]), st["Heading2"])]
        elif l.strip(): f.append(Paragraph(escape(l.lstrip("- `").rstrip("`"))[:200], st["Code"] if l.startswith("-") else st["BodyText"]))
    doc.build(f); print("pdf ok")
except ImportError: print("reportlab missing: markdown only")
print({t:len(r) for t,_,r in sec})
