use retinacare_domain::{Examination, Finding, Patient};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportData {
    pub patient: Patient,
    pub examination: Examination,
    pub findings: Vec<Finding>,
    pub model_id: Option<String>,
    pub clinical_review_required: bool,
}

impl ReportData {
    pub fn from_records(
        patient: Patient,
        examination: Examination,
        findings: Vec<Finding>,
        model_id: Option<String>,
    ) -> Self {
        Self {
            patient,
            examination,
            findings,
            model_id,
            clinical_review_required: true,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// PDF renderer. With fonts installed (`install_fonts`) text is written as embedded TrueType
// (Type0 / Identity-H) with a ToUnicode map, so it stays selectable and extractable; without
// fonts it falls back to the standard Helvetica pair (Latin-1 only, anything else is '?').
// ---------------------------------------------------------------------------------------------

use flate2::{Compression, write::ZlibEncoder};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Write as _,
    path::{Path, PathBuf},
    sync::OnceLock,
};
use ttf_parser::{Face, GlyphId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableColumn {
    pub title: String,
    /// Relative width; columns share the text width in proportion.
    pub weight: u16,
    pub right_aligned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PdfLine {
    Heading(String),
    Text(String),
    /// Bold warning-style line (research-only banner, limitations).
    Banner(String),
    Blank,
    /// Continue on a new page (used to keep technical provenance apart from the summary).
    PageBreak,
    /// Ruled table; the header repeats when it continues on another page.
    Table {
        columns: Vec<TableColumn>,
        rows: Vec<Vec<String>>,
    },
    /// 8-bit RGB picture on its own page (`rgb.len() == width * height * 3`), with a caption above it.
    Image {
        caption: String,
        width: u32,
        height: u32,
        rgb: Vec<u8>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PdfDocument {
    pub title: String,
    pub lines: Vec<PdfLine>,
}

#[derive(Debug)]
pub enum ReportError {
    EmptyDocument,
    Io(std::io::Error),
    Invalid(&'static str),
}

impl std::fmt::Display for ReportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyDocument => write!(f, "report has no content"),
            Self::Io(error) => write!(f, "could not write the report: {error}"),
            Self::Invalid(why) => write!(f, "generated PDF failed validation: {why}"),
        }
    }
}

impl std::error::Error for ReportError {}

impl From<std::io::Error> for ReportError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

const PAGE_WIDTH: f32 = 595.0;
const PAGE_HEIGHT: f32 = 842.0;
const MARGIN: f32 = 56.0;
const LINE_HEIGHT: f32 = 16.0;
const BODY_SIZE: f32 = 11.0;
const TABLE_SIZE: f32 = 10.0;
const HEADING_SIZE: f32 = 14.0;
const TEXT_WIDTH: f32 = PAGE_WIDTH - 2.0 * MARGIN;
const CELL_PAD: f32 = 6.0;

// --- fonts -------------------------------------------------------------------------------------

struct Fonts {
    regular: Face<'static>,
    bold: Face<'static>,
    regular_bytes: &'static [u8],
    bold_bytes: &'static [u8],
}

static FONTS: OnceLock<Fonts> = OnceLock::new();

/// Install the TrueType faces used for every report (process-wide, first call wins).
/// Fails when either file is not a parseable font.
pub fn install_fonts(regular: Vec<u8>, bold: Vec<u8>) -> Result<(), ReportError> {
    let regular: &'static [u8] = Box::leak(regular.into_boxed_slice());
    let bold: &'static [u8] = Box::leak(bold.into_boxed_slice());
    let parse = |b: &'static [u8]| Face::parse(b, 0).map_err(|_| ReportError::Invalid("font"));
    let fonts = Fonts {
        regular: parse(regular)?,
        bold: parse(bold)?,
        regular_bytes: regular,
        bold_bytes: bold,
    };
    let _ = FONTS.set(fonts);
    Ok(())
}

fn char_width(c: char, bold: bool, size: f32) -> f32 {
    match FONTS.get() {
        Some(f) => {
            let face = if bold { &f.bold } else { &f.regular };
            let gid = face.glyph_index(c).unwrap_or(GlyphId(0));
            let advance = face.glyph_hor_advance(gid).unwrap_or(0);
            f32::from(advance) / f32::from(face.units_per_em()) * size
        }
        None => 0.52 * size,
    }
}

fn text_width(text: &str, bold: bool, size: f32) -> f32 {
    text.chars().map(|c| char_width(c, bold, size)).sum()
}

/// Characters the installed fonts cannot draw (they would show as the font's missing-glyph box).
/// Without installed fonts, everything outside Latin-1 is reported.
pub fn unsupported_chars(document: &PdfDocument) -> Vec<char> {
    let mut bad = Vec::new();
    let mut check = |text: &str| {
        for c in text.chars().filter(|c| !c.is_whitespace()) {
            let ok = match FONTS.get() {
                Some(f) => f.regular.glyph_index(c).is_some() && f.bold.glyph_index(c).is_some(),
                None => {
                    (c.is_ascii() && !c.is_ascii_control())
                        || ((c as u32) >= 0xA0 && (c as u32) < 256)
                }
            };
            if !ok && !bad.contains(&c) {
                bad.push(c);
            }
        }
    };
    check(&document.title);
    for line in &document.lines {
        match line {
            PdfLine::Heading(t) | PdfLine::Text(t) | PdfLine::Banner(t) => check(t),
            PdfLine::Table { columns, rows } => {
                columns.iter().for_each(|c| check(&c.title));
                rows.iter().flatten().for_each(|c| check(c));
            }
            PdfLine::Image { caption, .. } => check(caption),
            PdfLine::Blank | PdfLine::PageBreak => {}
        }
    }
    bad
}

// --- layout ------------------------------------------------------------------------------------

fn sanitise(text: &str) -> String {
    text.replace('\\', "/")
}

/// Wrap by measured glyph width. A word wider than the line is split at the character that overflows.
fn wrap(text: &str, bold: bool, size: f32, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_w = 0.0_f32;
    let space = char_width(' ', bold, size);
    for word in text.split_whitespace() {
        let mut word = word.to_string();
        while text_width(&word, bold, size) > max_width {
            let mut head = String::new();
            let mut w = 0.0;
            let mut taken = 0;
            for c in word.chars() {
                let cw = char_width(c, bold, size);
                if w + cw > max_width && taken > 0 {
                    break;
                }
                head.push(c);
                w += cw;
                taken += 1;
            }
            word = word.chars().skip(taken).collect();
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            lines.push(head);
            current_w = 0.0;
        }
        let word_w = text_width(&word, bold, size);
        if !current.is_empty() && current_w + space + word_w > max_width {
            lines.push(std::mem::take(&mut current));
            current_w = 0.0;
        }
        if !current.is_empty() {
            current.push(' ');
            current_w += space;
        }
        current.push_str(&word);
        current_w += word_w;
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

/// One piece of text (or a horizontal rule) on a page row. `x` is the offset from the left margin.
#[derive(Clone, Debug, PartialEq)]
pub struct PageRow {
    pub bold: bool,
    pub size: f32,
    pub text: String,
    pub row: usize,
    pub x: f32,
    /// A thin line of this width under the row instead of text.
    pub rule: Option<f32>,
}

/// A page exactly as `render_pdf` lays it out, so a screen preview matches the file.
pub enum PreviewPage<'a> {
    Text(Vec<PageRow>),
    Picture {
        caption: Vec<PageRow>,
        width: u32,
        height: u32,
        rgb: &'a [u8],
    },
}

/// A4 size in points and text margin used by the layout.
pub const PAGE_SIZE: (f32, f32) = (PAGE_WIDTH, PAGE_HEIGHT);
pub const PAGE_MARGIN: f32 = MARGIN;
pub const ROW_HEIGHT: f32 = LINE_HEIGHT;
pub const ROWS_PER_PAGE: usize = ((PAGE_HEIGHT - 2.0 * MARGIN) / LINE_HEIGHT) as usize;

struct Layout {
    pages: Vec<Vec<PageRow>>,
    row: usize,
}

impl Layout {
    fn new() -> Self {
        Self {
            pages: vec![Vec::new()],
            row: 0,
        }
    }
    fn new_page(&mut self) {
        if self.row > 0 {
            self.pages.push(Vec::new());
            self.row = 0;
        }
    }
    fn ensure(&mut self, rows: usize) {
        if self.row + rows > ROWS_PER_PAGE {
            self.new_page();
        }
    }
    fn text(&mut self, bold: bool, size: f32, x: f32, text: String) {
        let row = self.row;
        self.put(bold, size, x, text, row, None);
    }
    fn put(&mut self, bold: bool, size: f32, x: f32, text: String, row: usize, rule: Option<f32>) {
        if let Some(page) = self.pages.last_mut() {
            page.push(PageRow {
                bold,
                size,
                text,
                row,
                x,
                rule,
            });
        }
    }
    fn paragraph(&mut self, bold: bool, size: f32, text: &str, indent: f32) {
        for piece in wrap(&sanitise(text), bold, size, TEXT_WIDTH - indent) {
            self.ensure(1);
            self.text(bold, size, indent, piece);
            self.row += 1;
        }
    }
    fn table(&mut self, columns: &[TableColumn], rows: &[Vec<String>]) {
        let total: f32 = columns.iter().map(|c| f32::from(c.weight.max(1))).sum();
        let mut starts = Vec::new();
        let mut widths = Vec::new();
        let mut x = 0.0;
        for c in columns {
            let w = TEXT_WIDTH * f32::from(c.weight.max(1)) / total;
            starts.push(x);
            widths.push(w);
            x += w;
        }
        let header = |l: &mut Layout| {
            l.ensure(3);
            let r = l.row;
            for (i, c) in columns.iter().enumerate() {
                let t = sanitise(&c.title);
                let x = cell_x(starts[i], widths[i], &t, true, TABLE_SIZE, c.right_aligned);
                l.put(true, TABLE_SIZE, x, t, r, None);
            }
            l.put(false, TABLE_SIZE, 0.0, String::new(), r, Some(TEXT_WIDTH));
            l.row += 1;
        };
        header(self);
        for data in rows {
            let cells: Vec<Vec<String>> = columns
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let t = data.get(i).map_or("", String::as_str);
                    wrap(&sanitise(t), false, TABLE_SIZE, widths[i] - 2.0 * CELL_PAD)
                })
                .collect();
            let height = cells.iter().map(Vec::len).max().unwrap_or(1);
            if self.row + height > ROWS_PER_PAGE {
                self.new_page();
                header(self);
            }
            let r0 = self.row;
            for (i, lines) in cells.iter().enumerate() {
                for (j, line) in lines.iter().enumerate() {
                    let x = cell_x(
                        starts[i] + CELL_PAD,
                        widths[i] - 2.0 * CELL_PAD,
                        line,
                        false,
                        TABLE_SIZE,
                        columns[i].right_aligned,
                    );
                    self.put(false, TABLE_SIZE, x, line.clone(), r0 + j, None);
                }
            }
            self.put(
                false,
                TABLE_SIZE,
                0.0,
                String::new(),
                r0 + height - 1,
                Some(TEXT_WIDTH),
            );
            self.row += height;
        }
    }
}

fn cell_x(start: f32, width: f32, text: &str, bold: bool, size: f32, right: bool) -> f32 {
    if right {
        (start + width - text_width(text, bold, size)).max(start)
    } else {
        start
    }
}

/// Lays the document out into pages: text pages first, then one page per picture.
pub fn preview_pages(document: &PdfDocument) -> Result<Vec<PreviewPage<'_>>, ReportError> {
    if document.lines.is_empty() {
        return Err(ReportError::EmptyDocument);
    }
    let mut layout = Layout::new();
    layout.paragraph(true, 20.0, &document.title, 0.0);
    layout.row += 1;
    let mut pictures = Vec::new();
    for line in &document.lines {
        match line {
            PdfLine::Image {
                caption,
                width,
                height,
                rgb,
            } => {
                if *width == 0
                    || *height == 0
                    || *width > 8000
                    || *height > 8000
                    || rgb.len() != *width as usize * *height as usize * 3
                {
                    return Err(ReportError::Invalid("embedded image has inconsistent size"));
                }
                let caption_rows: Vec<PageRow> =
                    wrap(&sanitise(caption), false, BODY_SIZE, TEXT_WIDTH)
                        .into_iter()
                        .enumerate()
                        .map(|(row, text)| PageRow {
                            bold: false,
                            size: BODY_SIZE - 1.0,
                            text,
                            row,
                            x: 0.0,
                            rule: None,
                        })
                        .collect();
                pictures.push((caption_rows, *width, *height, rgb.as_slice()));
            }
            PdfLine::Heading(t) => {
                layout.ensure(3);
                layout.paragraph(true, HEADING_SIZE, t, 0.0);
            }
            PdfLine::Text(t) => layout.paragraph(false, BODY_SIZE, t, 0.0),
            PdfLine::Banner(t) => layout.paragraph(true, BODY_SIZE, t, 0.0),
            PdfLine::Blank => {
                if layout.row > 0 {
                    layout.row += 1;
                }
            }
            PdfLine::PageBreak => layout.new_page(),
            PdfLine::Table { columns, rows } => layout.table(columns, rows),
        }
    }
    let mut pages: Vec<PreviewPage> = layout
        .pages
        .into_iter()
        .filter(|p| !p.is_empty())
        .map(PreviewPage::Text)
        .collect();
    for (caption, width, height, rgb) in pictures {
        pages.push(PreviewPage::Picture {
            caption,
            width,
            height,
            rgb,
        });
    }
    Ok(pages)
}

// --- writer ------------------------------------------------------------------------------------

struct Pdf {
    objects: Vec<Vec<u8>>,
}

impl Pdf {
    fn add(&mut self, body: Vec<u8>) -> usize {
        self.objects.push(body);
        self.objects.len()
    }
    fn set(&mut self, id: usize, body: Vec<u8>) {
        self.objects[id - 1] = body;
    }
    fn stream(&mut self, dict: &str, data: &[u8]) -> usize {
        let mut body = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
        body.extend_from_slice(data);
        body.extend_from_slice(b"\nendstream");
        self.add(body)
    }
}

fn deflate(data: &[u8]) -> Result<Vec<u8>, ReportError> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

/// Helvetica fallback: Latin-1 as WinAnsi, everything else '?'.
fn latin1_literal(text: &str) -> String {
    let mut out = String::from("(");
    for c in text.chars() {
        match c {
            '(' | ')' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            c if c.is_ascii() && !c.is_ascii_control() => out.push(c),
            c if (0xA0..256).contains(&(c as u32)) => out.push_str(&format!("\\{:03o}", c as u32)),
            _ => out.push('?'),
        }
    }
    out.push(')');
    out
}

/// Glyph ids used per face, with the character each came from (for widths and ToUnicode).
#[derive(Default)]
struct Used {
    glyphs: [BTreeMap<u16, char>; 2],
}

fn show(text: &str, bold: bool, used: &mut Used) -> String {
    match FONTS.get() {
        Some(f) => {
            let face = if bold { &f.bold } else { &f.regular };
            let mut hex = String::from("<");
            for c in text.chars() {
                let gid = face.glyph_index(c).map_or(0, |g| g.0);
                if gid != 0 {
                    used.glyphs[usize::from(bold)].insert(gid, c);
                }
                hex.push_str(&format!("{gid:04x}"));
            }
            hex.push('>');
            hex
        }
        None => latin1_literal(text),
    }
}

fn to_unicode_cmap(map: &BTreeMap<u16, char>) -> String {
    let mut out = String::from(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Adobe-Identity-UCS def /CMapType 2 def\n1 begincodespacerange <0000> <FFFF> endcodespacerange\n",
    );
    let entries: Vec<_> = map.iter().collect();
    for chunk in entries.chunks(100) {
        out.push_str(&format!("{} beginbfchar\n", chunk.len()));
        for (gid, c) in chunk {
            let mut units = [0u16; 2];
            let encoded = c.encode_utf16(&mut units);
            let hex: String = encoded.iter().map(|u| format!("{u:04X}")).collect();
            out.push_str(&format!("<{gid:04X}> <{hex}>\n"));
        }
        out.push_str("endbfchar\n");
    }
    out.push_str("endcmap CMapName currentdict /CMap defineresource pop end end\n");
    out
}

fn embed_face(
    pdf: &mut Pdf,
    type0_id: usize,
    name: &str,
    face: &Face<'static>,
    bytes: &[u8],
    used: &BTreeMap<u16, char>,
) -> Result<(), ReportError> {
    let upm = f32::from(face.units_per_em());
    let scale = |v: i16| (f32::from(v) / upm * 1000.0).round() as i32;
    let file = pdf.stream(
        &format!("/Filter /FlateDecode /Length1 {}", bytes.len()),
        &deflate(bytes)?,
    );
    let bbox = face.global_bounding_box();
    let descriptor = pdf.add(
        format!(
            "<< /Type /FontDescriptor /FontName /{name} /Flags 32 /FontBBox [{} {} {} {}] /ItalicAngle 0 /Ascent {} /Descent {} /CapHeight {} /StemV 80 /FontFile2 {file} 0 R >>",
            scale(bbox.x_min),
            scale(bbox.y_min),
            scale(bbox.x_max),
            scale(bbox.y_max),
            scale(face.ascender()),
            scale(face.descender()),
            scale(face.capital_height().unwrap_or(700)),
        )
        .into_bytes(),
    );
    let widths: String = used
        .keys()
        .map(|gid| {
            let adv = face.glyph_hor_advance(GlyphId(*gid)).unwrap_or(0) as f32 / upm * 1000.0;
            format!("{gid} [{}]", adv.round() as i32)
        })
        .collect::<Vec<_>>()
        .join(" ");
    let descendant = pdf.add(
        format!(
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{name} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {descriptor} 0 R /CIDToGIDMap /Identity /DW 600 /W [{widths}] >>"
        )
        .into_bytes(),
    );
    let cmap = pdf.stream("", to_unicode_cmap(used).as_bytes());
    pdf.set(
        type0_id,
        format!(
            "<< /Type /Font /Subtype /Type0 /BaseFont /{name} /Encoding /Identity-H /DescendantFonts [{descendant} 0 R] /ToUnicode {cmap} 0 R >>"
        )
        .into_bytes(),
    );
    Ok(())
}

/// Render the document to PDF bytes (PDF 1.4, A4).
pub fn render_pdf(document: &PdfDocument) -> Result<Vec<u8>, ReportError> {
    let pages = preview_pages(document)?;
    let mut pdf = Pdf {
        objects: Vec::new(),
    };
    let catalog = pdf.add(Vec::new());
    let pages_id = pdf.add(Vec::new());
    let font_regular = pdf.add(Vec::new());
    let font_bold = pdf.add(Vec::new());
    let mut used = Used::default();
    let mut kids = Vec::new();
    let total = pages.len();
    for (index, page) in pages.iter().enumerate() {
        let (rows, picture): (&[PageRow], _) = match page {
            PreviewPage::Text(rows) => (rows, None),
            PreviewPage::Picture {
                caption,
                width,
                height,
                rgb,
            } => (caption, Some((*width, *height, *rgb))),
        };
        let mut stream = String::new();
        let mut draw = |stream: &mut String, row: &PageRow| {
            let y = PAGE_HEIGHT - MARGIN - LINE_HEIGHT * (row.row as f32 + 1.0);
            if let Some(width) = row.rule {
                stream.push_str(&format!(
                    "0.75 G 0.5 w {MARGIN} {} m {} {} l S 0 G\n",
                    y - 4.0,
                    MARGIN + width,
                    y - 4.0
                ));
            } else if !row.text.is_empty() {
                stream.push_str(&format!(
                    "BT /{} {} Tf {} {y} Td {} Tj ET\n",
                    if row.bold { "F2" } else { "F1" },
                    row.size,
                    MARGIN + row.x,
                    show(&row.text, row.bold, &mut used)
                ));
            }
        };
        for row in rows {
            draw(&mut stream, row);
        }
        let mut xobject = String::new();
        if let Some((width, height, rgb)) = picture {
            let id = pdf.stream(
                &format!(
                    "/Type /XObject /Subtype /Image /Width {width} /Height {height} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode"
                ),
                &deflate(rgb)?,
            );
            xobject = format!(" /XObject << /Im0 {id} 0 R >>");
            let top = PAGE_HEIGHT - MARGIN - LINE_HEIGHT * (rows.len() as f32 + 1.5);
            let scale = (TEXT_WIDTH / width as f32).min((top - 60.0) / height as f32);
            let (w, h) = (width as f32 * scale, height as f32 * scale);
            stream.push_str(&format!(
                "q {w} 0 0 {h} {} {} cm /Im0 Do Q\n",
                MARGIN + (TEXT_WIDTH - w) / 2.0,
                top - h
            ));
        }
        let footer = PageRow {
            bold: false,
            size: 8.0,
            text: format!(
                "Page {} of {total} - research prototype report, not a clinical document",
                index + 1
            ),
            row: 0,
            x: 0.0,
            rule: None,
        };
        stream.push_str(&format!(
            "BT /F1 8 Tf {MARGIN} 28 Td {} Tj ET\n",
            show(&footer.text, false, &mut used)
        ));
        let content = pdf.stream("/Filter /FlateDecode", &deflate(stream.as_bytes())?);
        let page_id = pdf.add(
            format!(
                "<< /Type /Page /Parent {pages_id} 0 R /MediaBox [0 0 {PAGE_WIDTH} {PAGE_HEIGHT}] /Contents {content} 0 R /Resources << /Font << /F1 {font_regular} 0 R /F2 {font_bold} 0 R >>{xobject} >> >>"
            )
            .into_bytes(),
        );
        kids.push(format!("{page_id} 0 R"));
    }
    pdf.set(
        catalog,
        format!("<< /Type /Catalog /Pages {pages_id} 0 R >>").into_bytes(),
    );
    pdf.set(
        pages_id,
        format!(
            "<< /Type /Pages /Kids [{}] /Count {total} >>",
            kids.join(" ")
        )
        .into_bytes(),
    );
    match FONTS.get() {
        Some(f) => {
            embed_face(
                &mut pdf,
                font_regular,
                "ReportSans",
                &f.regular,
                f.regular_bytes,
                &used.glyphs[0],
            )?;
            embed_face(
                &mut pdf,
                font_bold,
                "ReportSans-Bold",
                &f.bold,
                f.bold_bytes,
                &used.glyphs[1],
            )?;
        }
        None => {
            pdf.set(font_regular, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec());
            pdf.set(font_bold, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_vec());
        }
    }

    let mut bytes: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in pdf.objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        bytes.extend_from_slice(body);
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let xref = bytes.len();
    bytes.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", pdf.objects.len() + 1).as_bytes(),
    );
    for offset in &offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            pdf.objects.len() + 1
        )
        .as_bytes(),
    );
    validate_pdf(&bytes)?;
    Ok(bytes)
}

fn find_last(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).rposition(|w| w == needle)
}

/// Structural self-check: header, trailer, and every xref offset points at its object.
pub fn validate_pdf(bytes: &[u8]) -> Result<(), ReportError> {
    if !bytes.starts_with(b"%PDF-1.") {
        return Err(ReportError::Invalid("missing header"));
    }
    if !bytes.ends_with(b"%%EOF\n") {
        return Err(ReportError::Invalid("missing end-of-file marker"));
    }
    let start =
        find_last(bytes, b"startxref\n").ok_or(ReportError::Invalid("missing startxref"))?;
    let tail = String::from_utf8_lossy(&bytes[start + 10..]);
    let xref_offset: usize = tail
        .lines()
        .next()
        .and_then(|l| l.trim().parse().ok())
        .ok_or(ReportError::Invalid("bad startxref"))?;
    if !bytes
        .get(xref_offset..)
        .is_some_and(|b| b.starts_with(b"xref\n"))
    {
        return Err(ReportError::Invalid("xref offset is wrong"));
    }
    let table = String::from_utf8_lossy(&bytes[xref_offset..]);
    let mut lines = table.lines().skip(1);
    let count: usize = lines
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|n| n.parse().ok())
        .ok_or(ReportError::Invalid("bad xref header"))?;
    lines.next();
    for number in 1..count {
        let entry = lines.next().ok_or(ReportError::Invalid("short xref"))?;
        let offset: usize = entry
            .get(..10)
            .and_then(|e| e.parse().ok())
            .ok_or(ReportError::Invalid("bad xref entry"))?;
        let marker = format!("{number} 0 obj");
        if !bytes
            .get(offset..)
            .is_some_and(|b| b.starts_with(marker.as_bytes()))
        {
            return Err(ReportError::Invalid(
                "xref entry does not point at its object",
            ));
        }
    }
    Ok(())
}

/// Write the PDF atomically: temp file in the same directory, fsync, validate, then rename.
/// A failure never leaves a partial or invalid PDF at `destination`.
pub fn write_pdf_atomic(destination: &Path, document: &PdfDocument) -> Result<usize, ReportError> {
    let bytes = render_pdf(document)?;
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&parent)?;
    let mut temp_name = destination.file_name().unwrap_or_default().to_os_string();
    temp_name.push(format!(".{}.tmp", std::process::id()));
    let temp = parent.join(temp_name);
    let written = (|| -> Result<(), ReportError> {
        let mut file = File::create(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        validate_pdf(&fs::read(&temp)?)?;
        Ok(())
    })();
    match written.and_then(|()| fs::rename(&temp, destination).map_err(ReportError::from)) {
        Ok(()) => Ok(bytes.len()),
        Err(error) => {
            let _ = fs::remove_file(&temp);
            Err(error)
        }
    }
}

#[cfg(test)]
mod pdf_tests {
    use super::*;

    fn doc() -> PdfDocument {
        PdfDocument {
            title: "Report (test) \\ title".into(),
            lines: vec![
                PdfLine::Banner("RESEARCH ONLY".into()),
                PdfLine::Heading("Section".into()),
                PdfLine::Text("word ".repeat(400)),
                PdfLine::Blank,
                PdfLine::Text("non-latin 患者 and café".into()),
            ],
        }
    }

    fn table(rows: usize) -> PdfLine {
        let col = |t: &str, weight, right_aligned| TableColumn {
            title: t.into(),
            weight,
            right_aligned,
        };
        PdfLine::Table {
            columns: vec![
                col("Structure", 4, false),
                col("Pixels", 2, true),
                col("% of image", 2, true),
            ],
            rows: (0..rows)
                .map(|i| {
                    vec![
                        format!("Structure {i}"),
                        format!("{}", i * 1000),
                        format!("{:.3}", i as f32 / 7.0),
                    ]
                })
                .collect(),
        }
    }

    fn rows_of(page: &PreviewPage) -> Vec<PageRow> {
        match page {
            PreviewPage::Text(rows) => rows.clone(),
            PreviewPage::Picture { caption, .. } => caption.clone(),
        }
    }

    #[test]
    fn rendered_pdf_is_structurally_valid_and_multi_page() {
        let bytes = render_pdf(&doc()).unwrap();
        validate_pdf(&bytes).unwrap();
        let mut long = doc();
        long.lines
            .extend((0..200).map(|i| PdfLine::Text(format!("row {i}"))));
        let many = render_pdf(&long).unwrap();
        validate_pdf(&many).unwrap();
        assert!(preview_pages(&long).unwrap().len() > 2);
    }

    #[test]
    fn page_break_starts_a_new_page_without_a_blank_one() {
        let d = PdfDocument {
            title: "t".into(),
            lines: vec![
                PdfLine::Text("first".into()),
                PdfLine::PageBreak,
                PdfLine::PageBreak,
                PdfLine::Text("second".into()),
            ],
        };
        let pages = preview_pages(&d).unwrap();
        assert_eq!(pages.len(), 2);
        let rows = rows_of(&pages[1]);
        assert_eq!(rows[0].text, "second");
        assert_eq!(rows[0].row, 0);
        validate_pdf(&render_pdf(&d).unwrap()).unwrap();
    }

    #[test]
    fn table_rows_wrap_in_their_column_and_header_repeats_across_pages() {
        let d = PdfDocument {
            title: "t".into(),
            lines: vec![table(120)],
        };
        let pages = preview_pages(&d).unwrap();
        assert!(pages.len() >= 3);
        for page in &pages {
            let rows = rows_of(page);
            assert!(rows.iter().all(|r| r.row < ROWS_PER_PAGE));
            assert!(
                rows.iter().any(|r| r.text == "Structure" && r.bold),
                "header on every page"
            );
        }
        let first = rows_of(&pages[0]);
        let pixels_header = first.iter().find(|r| r.text == "Pixels").unwrap();
        let pixels_cell = first.iter().find(|r| r.text == "1000").unwrap();
        // right aligned: the value ends where the header ends
        let end = |r: &PageRow| r.x + text_width(&r.text, r.bold, r.size);
        assert!((end(pixels_header) - end(pixels_cell)).abs() < 0.5 + CELL_PAD);
        validate_pdf(&render_pdf(&d).unwrap()).unwrap();
        let long = PdfDocument {
            title: "t".into(),
            lines: vec![PdfLine::Table {
                columns: vec![TableColumn {
                    title: "A".into(),
                    weight: 1,
                    right_aligned: false,
                }],
                rows: vec![vec!["x".repeat(500)]],
            }],
        };
        let rows = rows_of(&preview_pages(&long).unwrap()[0]);
        assert!(rows.iter().filter(|r| r.text.starts_with('x')).count() > 1);
    }

    #[test]
    fn empty_table_still_renders_its_header() {
        let d = PdfDocument {
            title: "t".into(),
            lines: vec![table(0)],
        };
        validate_pdf(&render_pdf(&d).unwrap()).unwrap();
    }

    #[test]
    fn empty_document_and_corrupt_pdf_are_rejected() {
        let empty = PdfDocument {
            title: "t".into(),
            lines: vec![],
        };
        assert!(matches!(
            render_pdf(&empty),
            Err(ReportError::EmptyDocument)
        ));
        assert!(validate_pdf(b"not a pdf").is_err());
        let mut bytes = render_pdf(&doc()).unwrap();
        bytes.truncate(bytes.len() - 6);
        assert!(validate_pdf(&bytes).is_err());
    }

    #[test]
    fn atomic_write_creates_file_and_failure_leaves_nothing() {
        let dir = std::env::temp_dir().join(format!("rc-pdf-{}", std::process::id()));
        let target = dir.join("sub dir/report.pdf");
        let size = write_pdf_atomic(&target, &doc()).unwrap();
        assert_eq!(fs::metadata(&target).unwrap().len() as usize, size);
        validate_pdf(&fs::read(&target).unwrap()).unwrap();
        let blocked = dir.join("blocked.pdf");
        fs::create_dir_all(&blocked).unwrap();
        assert!(write_pdf_atomic(&blocked, &doc()).is_err());
        let leftovers = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn embedded_picture_page_is_valid_and_rejects_bad_sizes() {
        let mut d = doc();
        d.lines.push(PdfLine::Image {
            caption: "Overlay (synthetic test pattern)".into(),
            width: 4,
            height: 2,
            rgb: (0..24).collect(),
        });
        let bytes = render_pdf(&d).unwrap();
        validate_pdf(&bytes).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("/Subtype /Image") && text.contains("/FlateDecode"));
        d.lines.pop();
        d.lines.push(PdfLine::Image {
            caption: "bad".into(),
            width: 4,
            height: 2,
            rgb: vec![0; 5],
        });
        assert!(matches!(render_pdf(&d), Err(ReportError::Invalid(_))));
    }

    #[test]
    fn installed_font_embeds_with_tounicode_and_reports_missing_glyphs() {
        let dir = "/usr/share/fonts/truetype/dejavu";
        let (Ok(regular), Ok(bold)) = (
            fs::read(format!("{dir}/DejaVuSans.ttf")),
            fs::read(format!("{dir}/DejaVuSans-Bold.ttf")),
        ) else {
            return; // font not present on this machine: nothing to measure
        };
        install_fonts(regular, bold).unwrap();
        let d = PdfDocument {
            title: "Caf\u{e9} \u{2014} \u{b0} \u{3b1}".into(),
            lines: vec![PdfLine::Text("\u{2013} \u{2026} \u{91c7}\u{8005}".into())],
        };
        assert_eq!(unsupported_chars(&d), vec!['\u{91c7}', '\u{8005}']);
        let bytes = render_pdf(&d).unwrap();
        validate_pdf(&bytes).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("/Subtype /Type0")
                && text.contains("/FontFile2")
                && text.contains("/ToUnicode")
        );
    }
}
