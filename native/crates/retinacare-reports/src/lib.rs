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
// Minimal PDF renderer: text-only, standard Helvetica fonts, no external dependencies.
// Characters outside Latin-1 are written as '?' (standard PDF fonts cannot encode them).
// ---------------------------------------------------------------------------------------------

use std::{
    fmt::Write as _,
    fs::{self, File},
    io::Write as _,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PdfLine {
    Heading(String),
    Text(String),
    /// Bold warning-style line (research-only banner, limitations).
    Banner(String),
    Blank,
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
const MARGIN: f32 = 50.0;
const LINE_HEIGHT: f32 = 14.0;
const WRAP_COLUMNS: usize = 92;

fn latin1(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\\' | '(' | ')' => c.to_string(),
            c if c.is_ascii() && !c.is_ascii_control() => c.to_string(),
            c if (c as u32) < 256 && (c as u32) >= 0xA0 => format!("\\{:03o}", c as u32),
            _ => "?".to_string(),
        })
        .collect()
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for part in latin1(text).chars() {
        match part {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            other => out.push(other),
        }
    }
    // latin1() leaves backslashes untouched only as a literal escape introducer for octal bytes;
    // a user-supplied backslash is doubled below before escapes were inserted.
    out
}

fn sanitise(text: &str) -> String {
    text.replace('\\', "/")
}

fn wrap(text: &str, columns: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let mut word = word.to_string();
        while word.chars().count() > columns {
            let head: String = word.chars().take(columns).collect();
            word = word.chars().skip(columns).collect();
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            lines.push(head);
        }
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > columns {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(&word);
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

/// Render the document to PDF bytes (PDF 1.4, A4, Helvetica / Helvetica-Bold).
/// One wrapped line of a laid-out page.
#[derive(Clone, Debug, PartialEq)]
pub struct PageRow {
    pub bold: bool,
    pub size: f32,
    pub text: String,
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

/// Lays the document out into pages: text pages first, then one page per picture.
pub fn preview_pages(document: &PdfDocument) -> Result<Vec<PreviewPage<'_>>, ReportError> {
    if document.lines.is_empty() {
        return Err(ReportError::EmptyDocument);
    }
    let row = |bold, size, text| PageRow { bold, size, text };
    let mut rows = vec![
        row(true, 16.0, sanitise(&document.title)),
        row(false, 10.0, String::new()),
    ];
    let per_page = ((PAGE_HEIGHT - 2.0 * MARGIN) / LINE_HEIGHT) as usize;
    let mut pictures = Vec::new();
    for line in &document.lines {
        if let PdfLine::Image {
            caption,
            width,
            height,
            rgb,
        } = line
        {
            if *width == 0
                || *height == 0
                || *width > 4000
                || *height > 4000
                || rgb.len() != *width as usize * *height as usize * 3
            {
                return Err(ReportError::Invalid("embedded image has inconsistent size"));
            }
            let caption_rows: Vec<PageRow> = wrap(&sanitise(caption), WRAP_COLUMNS)
                .into_iter()
                .map(|text| row(false, 10.0, text))
                .collect();
            // Pictures get their own pages after all text rows.
            pictures.push((caption_rows, *width, *height, rgb.as_slice()));
            continue;
        }
        let (bold, size, text, columns) = match line {
            PdfLine::Heading(t) => (true, 12.0, t.as_str(), WRAP_COLUMNS - 12),
            PdfLine::Text(t) => (false, 10.0, t.as_str(), WRAP_COLUMNS),
            PdfLine::Banner(t) => (true, 10.0, t.as_str(), WRAP_COLUMNS - 6),
            PdfLine::Blank => (false, 10.0, "", WRAP_COLUMNS),
            PdfLine::Image { .. } => unreachable!(),
        };
        for piece in wrap(&sanitise(text), columns) {
            rows.push(row(bold, size, piece));
        }
    }
    let mut pages: Vec<PreviewPage> = rows
        .chunks(per_page)
        .map(|c| PreviewPage::Text(c.to_vec()))
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

/// Render the document to PDF bytes (PDF 1.4, A4, Helvetica / Helvetica-Bold).
pub fn render_pdf(document: &PdfDocument) -> Result<Vec<u8>, ReportError> {
    let pages = preview_pages(document)?;
    let picture_count = pages
        .iter()
        .filter(|p| matches!(p, PreviewPage::Picture { .. }))
        .count();

    let mut objects: Vec<String> = Vec::new();
    objects.push("<< /Type /Catalog /Pages 2 0 R >>".into());
    let first_page_id = 5 + picture_count;
    let kids: String = (0..pages.len())
        .map(|i| format!("{} 0 R", first_page_id + 2 * i))
        .collect::<Vec<_>>()
        .join(" ");
    objects.push(format!(
        "<< /Type /Pages /Kids [{kids}] /Count {} >>",
        pages.len()
    ));
    objects.push(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".into(),
    );
    objects.push(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>"
            .into(),
    );
    // Image XObjects (ASCII-hex encoded so the whole file stays 7-bit and xref offsets stay exact).
    let mut image_ids: Vec<Option<usize>> = Vec::new();
    for page in &pages {
        if let PreviewPage::Picture {
            width, height, rgb, ..
        } = page
        {
            let mut hex = String::with_capacity(rgb.len() * 2 + 1);
            for byte in rgb.iter() {
                let _ = write!(hex, "{byte:02x}");
            }
            hex.push('>');
            image_ids.push(Some(5 + image_ids.iter().flatten().count()));
            objects.push(format!(
                "<< /Type /XObject /Subtype /Image /Width {width} /Height {height} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}\nendstream",
                hex.len()
            ));
        } else {
            image_ids.push(None);
        }
    }
    for (index, page) in pages.iter().enumerate() {
        let content_id = first_page_id + 1 + 2 * index;
        let xobject = image_ids[index]
            .map(|id| format!(" /XObject << /Im0 {id} 0 R >>"))
            .unwrap_or_default();
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH} {PAGE_HEIGHT}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >>{xobject} >> /Contents {content_id} 0 R >>"
        ));
        let mut stream = String::new();
        type Picture<'a> = Option<(&'a Vec<PageRow>, u32, u32)>;
        let (rows_here, picture): (&[PageRow], Picture) = match page {
            PreviewPage::Text(rows) => (rows, None),
            PreviewPage::Picture {
                caption,
                width,
                height,
                ..
            } => (&[], Some((caption, *width, *height))),
        };
        for (row_index, row) in rows_here.iter().enumerate() {
            let y = PAGE_HEIGHT - MARGIN - LINE_HEIGHT * (row_index as f32 + 1.0);
            let font = if row.bold { "F2" } else { "F1" };
            let _ = writeln!(
                stream,
                "BT /{font} {} Tf {MARGIN} {y} Td ({}) Tj ET",
                row.size,
                escape(&row.text)
            );
        }
        if let Some((caption, width, height)) = picture {
            for (row_index, row) in caption.iter().enumerate() {
                let y = PAGE_HEIGHT - MARGIN - LINE_HEIGHT * (row_index as f32 + 1.0);
                let _ = writeln!(
                    stream,
                    "BT /F1 {} Tf {MARGIN} {y} Td ({}) Tj ET",
                    row.size,
                    escape(&row.text)
                );
            }
            let max_w = PAGE_WIDTH - 2.0 * MARGIN;
            let top = PAGE_HEIGHT - MARGIN - LINE_HEIGHT * (caption.len() as f32 + 1.5);
            let max_h = top - 60.0;
            let scale = (max_w / width as f32).min(max_h / height as f32);
            let (w, h) = (width as f32 * scale, height as f32 * scale);
            let _ = writeln!(stream, "q {w} 0 0 {h} {MARGIN} {} cm /Im0 Do Q", top - h);
        }
        let _ = writeln!(
            stream,
            "BT /F1 8 Tf {MARGIN} 28 Td (Page {} of {} - research prototype report, not a clinical document) Tj ET",
            index + 1,
            pages.len()
        );
        objects.push(format!(
            "<< /Length {} >>\nstream\n{}endstream",
            stream.len(),
            stream
        ));
    }

    let mut bytes: Vec<u8> = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in &offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    validate_pdf(&bytes)?;
    Ok(bytes)
}

/// Structural self-check: header, trailer, and every xref offset points at its object.
pub fn validate_pdf(bytes: &[u8]) -> Result<(), ReportError> {
    if !bytes.starts_with(b"%PDF-1.") {
        return Err(ReportError::Invalid("missing header"));
    }
    if !bytes.ends_with(b"%%EOF\n") {
        return Err(ReportError::Invalid("missing end-of-file marker"));
    }
    let text = String::from_utf8_lossy(bytes);
    let start = text
        .rfind("startxref\n")
        .ok_or(ReportError::Invalid("missing startxref"))?;
    let xref_offset: usize = text[start + 10..]
        .lines()
        .next()
        .and_then(|l| l.trim().parse().ok())
        .ok_or(ReportError::Invalid("bad startxref"))?;
    if !text[xref_offset..].starts_with("xref\n") {
        return Err(ReportError::Invalid("xref offset is wrong"));
    }
    let mut lines = text[xref_offset..].lines().skip(1);
    let count: usize = lines
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|n| n.parse().ok())
        .ok_or(ReportError::Invalid("bad xref header"))?;
    lines.next();
    for number in 1..count {
        let entry = lines.next().ok_or(ReportError::Invalid("short xref"))?;
        let offset: usize = entry[..10]
            .parse()
            .map_err(|_| ReportError::Invalid("bad xref entry"))?;
        if !text[offset..].starts_with(&format!("{number} 0 obj")) {
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

    #[test]
    fn rendered_pdf_is_structurally_valid_and_multi_page() {
        let bytes = render_pdf(&doc()).unwrap();
        validate_pdf(&bytes).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("RESEARCH ONLY"));
        assert!(text.contains("non-latin ?? and caf"));
        let mut long = doc();
        long.lines
            .extend((0..200).map(|i| PdfLine::Text(format!("row {i}"))));
        let many = render_pdf(&long).unwrap();
        validate_pdf(&many).unwrap();
        assert!(!String::from_utf8_lossy(&many).contains("/Count 1 >>"));
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
        // destination is a directory: rename must fail and no temp file may remain
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
        assert!(bytes.is_ascii(), "image data must not break xref offsets");
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("/Subtype /Image") && text.contains("/Im0 Do"));
        assert!(text.contains("000102030405060708090a0b0c0d0e0f1011121314151617>"));
        d.lines.pop();
        d.lines.push(PdfLine::Image {
            caption: "bad".into(),
            width: 4,
            height: 2,
            rgb: vec![0; 5],
        });
        assert!(matches!(render_pdf(&d), Err(ReportError::Invalid(_))));
    }
}
