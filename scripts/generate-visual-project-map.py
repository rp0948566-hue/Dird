#!/usr/bin/env python3
from pathlib import Path

from reportlab.graphics import renderPDF
from reportlab.graphics.shapes import Drawing, Line, Polygon, Rect, String
from reportlab.lib import colors
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs/assets/retinacare-project-map.png"
PDF = ROOT / "docs/assets/retinacare-project-map.pdf"
FONTS = Path("/home/claude/.claude/skills/canvas-design/canvas-fonts")
pdfmetrics.registerFont(TTFont("Outfit", FONTS / "Outfit-Regular.ttf"))
pdfmetrics.registerFont(TTFont("Bricolage", FONTS / "BricolageGrotesque-Bold.ttf"))
pdfmetrics.registerFont(TTFont("PlexMono", FONTS / "IBMPlexMono-Bold.ttf"))

W, H = 2000, 1250
d = Drawing(W, H)
bg = colors.HexColor("#F4F2EC")
ink = colors.HexColor("#18232D")
muted = colors.HexColor("#61717E")
blue = colors.HexColor("#2878C7")
teal = colors.HexColor("#248A83")
amber = colors.HexColor("#D9922E")
violet = colors.HexColor("#7257A8")
red = colors.HexColor("#B64F4F")
white = colors.white
d.add(Rect(0, 0, W, H, fillColor=bg, strokeColor=None))
d.add(Rect(0, 1175, W, 75, fillColor=ink, strokeColor=None))
d.add(String(70, 1202, "RETINACARE", fontName="Bricolage", fontSize=34, fillColor=white))
d.add(String(1925, 1205, "NATIVE • OFFLINE • ENCRYPTED", textAnchor="end", fontName="PlexMono", fontSize=14, fillColor=colors.HexColor("#B8C8D4")))
d.add(String(70, 1125, "HOW THE PRODUCT, CODE, DATA AND AI CONNECT", fontName="Bricolage", fontSize=29, fillColor=ink))
d.add(String(70, 1088, "Blue = user flow   Teal = Rust control   Amber = protected data   Violet = AI   Red = still unverified", fontName="Outfit", fontSize=17, fillColor=muted))


def box(x, y, w, h, title, subtitle, color, title_size=19):
    d.add(Rect(x, y, w, h, 18, 18, fillColor=white, strokeColor=color, strokeWidth=2.5))
    d.add(Rect(x, y + h - 38, 8, 38, fillColor=color, strokeColor=None))
    d.add(String(x + 24, y + h - 29, title, fontName="Bricolage", fontSize=title_size, fillColor=ink))
    for i, line in enumerate(subtitle.split("\n")):
        d.add(String(x + 24, y + h - 61 - i * 23, line, fontName="Outfit", fontSize=14, fillColor=muted))


def arrow(x1, y1, x2, y2, color=muted, width=3):
    d.add(Line(x1, y1, x2, y2, strokeColor=color, strokeWidth=width))
    if abs(x2-x1) >= abs(y2-y1):
        direction = 1 if x2 > x1 else -1
        d.add(Polygon([x2, y2, x2-12*direction, y2+6, x2-12*direction, y2-6], fillColor=color, strokeColor=None))
    else:
        direction = 1 if y2 > y1 else -1
        d.add(Polygon([x2, y2, x2-6, y2-12*direction, x2+6, y2-12*direction], fillColor=color, strokeColor=None))


# User workflow ribbon
d.add(String(70, 1018, "01 / CLINICAL WORKFLOW", fontName="PlexMono", fontSize=15, fillColor=blue))
steps = [("PATIENT", "Add • edit • search"), ("SCREEN", "Eye • image • notes"), ("VIEW", "Zoom • pan • reset"), ("ANALYZE", "Worker request"), ("SAVE", "SQLCipher exam"), ("REOPEN", "History • report")]
for i, (title, subtitle) in enumerate(steps):
    x = 70 + i * 310
    box(x, 870, 250, 105, title, subtitle, blue, 17)
    if i < len(steps)-1: arrow(x+250, 922, x+300, 922, blue)

# Architecture layer
d.add(String(70, 808, "02 / WHO CALLS WHAT", fontName="PlexMono", fontSize=15, fillColor=teal))
box(70, 570, 300, 190, "NATIVE UI", "eframe / egui\n8 screens + file picker\nOwns visual state", blue, 23)
box(445, 570, 300, 190, "APPLICATION", "Rust controllers\nBounded background work\nOwns process lifecycle", teal, 23)
box(820, 570, 300, 190, "DOMAIN", "Patient • examination\nAnnotations • reports\nStrong typed contracts", colors.HexColor("#537287"), 23)
box(1195, 570, 300, 190, "SQLCIPHER", "SQLite encrypted at rest\nArgon2id-derived key\nNo generic SQL UI", amber, 23)
box(1570, 570, 300, 190, "LOCAL FILES", "Images stay local\nLogs + models + reports\nPer-user data folder", amber, 23)
for a, b in [(370,445),(745,820),(1120,1195),(1495,1570)]: arrow(a, 665, b, 665, teal)
d.add(String(405, 690, "events", fontName="PlexMono", fontSize=11, fillColor=teal))
d.add(String(775, 690, "types", fontName="PlexMono", fontSize=11, fillColor=teal))
d.add(String(1148, 690, "records", fontName="PlexMono", fontSize=11, fillColor=teal))

# AI lane
d.add(String(70, 508, "03 / AI PIPELINE", fontName="PlexMono", fontSize=15, fillColor=violet))
box(70, 310, 330, 145, "RUST AI BRIDGE", "Versioned JSON Lines\nTimeout • bounds • reap", violet, 21)
box(500, 310, 330, 145, "PYTHON WORKER", "Persistent subprocess\nHealth • preprocessing • timing", violet, 21)
box(930, 310, 330, 145, "PYTORCH / CUDA", "Device detection only\nRTX 5070 not yet verified", red, 21)
box(1360, 310, 510, 145, "MODEL RESULT CONTRACT", "Weights missing → explicit error\nNo fake diagnosis • findings • heatmap", red, 21)
arrow(400, 382, 500, 382, violet); arrow(830, 382, 930, 382, violet); arrow(1260, 382, 1360, 382, red)
d.add(String(446, 404, "stdin/stdout", fontName="PlexMono", fontSize=11, fillColor=violet))
d.add(String(1275, 404, "truthful", fontName="PlexMono", fontSize=11, fillColor=red))
arrow(595, 570, 235, 455, violet, 2.5)

# Status footer
d.add(String(70, 248, "04 / STATUS", fontName="PlexMono", fontSize=15, fillColor=ink))
done = ["Ubuntu GUI launched", "8-screen smoke passed", "12 Rust + 5 Python tests", "Encrypted patient/exam store", "Ubuntu + Windows packages"]
pending = ["Trained model adapter", "Windows execution", "RTX 5070 / CUDA timing", "Clinical PDF exporter", "Clinical validation"]
for i, item in enumerate(done):
    y = 192 - i * 31
    d.add(Rect(70, y-7, 14, 14, 7, 7, fillColor=teal, strokeColor=None)); d.add(String(100, y, item, fontName="Outfit", fontSize=14, fillColor=ink))
for i, item in enumerate(pending):
    y = 192 - i * 31
    d.add(Rect(650, y-7, 14, 14, 7, 7, fillColor=red, strokeColor=None)); d.add(String(680, y, item, fontName="Outfit", fontSize=14, fillColor=ink))

# Tech strip
d.add(Rect(1240, 45, 630, 170, 18, 18, fillColor=ink, strokeColor=None))
d.add(String(1270, 174, "TECH STACK", fontName="PlexMono", fontSize=14, fillColor=colors.HexColor("#B8C8D4")))
d.add(String(1270, 132, "RUST", fontName="Bricolage", fontSize=27, fillColor=white))
d.add(String(1410, 132, "PYTHON", fontName="Bricolage", fontSize=27, fillColor=white))
d.add(String(1585, 132, "SQLCIPHER", fontName="Bricolage", fontSize=27, fillColor=white))
d.add(String(1270, 92, "egui • eframe • rusqlite • Argon2id • Pillow • JSONL • Cargo • PowerShell", fontName="Outfit", fontSize=14, fillColor=colors.HexColor("#D7E1E8")))
d.add(String(1270, 65, "Legacy reference: React + TypeScript + Tauri   |   Final runtime: no WebView / HTTP server", fontName="Outfit", fontSize=13, fillColor=colors.HexColor("#93A8B7")))

OUT.parent.mkdir(parents=True, exist_ok=True)
renderPDF.drawToFile(d, str(PDF))
print(PDF)
