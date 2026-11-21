//! A minimal Word file (`.docx`): centred paragraphs of coloured text and centred pictures, one page per song. Nothing else is
//! needed by the songbook, so there is no general document model here: a document is a list of `Block`s.

use std::io::Write;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

/// A4 with the margins the songbook has always had (twips).
const PAGE_W: u32 = 11906;
const PAGE_H: u32 = 16838;
const MARGIN_SIDE: u32 = 900;
const MARGIN_TOP: u32 = 720;

/// A picture: its bytes, the extension Word expects, and its size in pixels at 96 per inch.
pub struct Picture {
    pub data: Vec<u8>,
    pub ext: &'static str,
    pub w: u32,
    pub h: u32,
}

/// One run of text.
pub struct Run {
    pub text: String,
    /// In half points (24 = 12 pt).
    pub size: u32,
    pub font: &'static str,
    pub bold: bool,
    pub italic: bool,
    /// Hex colour, no `#`.
    pub colour: &'static str,
}

pub enum Block {
    /// A centred paragraph. `before` and `after` are in twips; `page_break` starts a new page.
    Text { runs: Vec<Run>, before: u32, after: u32, page_break: bool },
    /// A paragraph that is empty.
    Blank { before: u32, after: u32, page_break: bool },
    /// A centred picture.
    Picture { picture: Picture, before: u32, after: u32 },
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            // a control character is not allowed in XML
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => {}
            c => out.push(c),
        }
    }
    out
}

fn run_xml(r: &Run) -> String {
    let font = r.font;
    let mut props = format!("<w:rFonts w:ascii=\"{font}\" w:hAnsi=\"{font}\" w:cs=\"{font}\"/>");
    if r.bold {
        props.push_str("<w:b/>");
    }
    if r.italic {
        props.push_str("<w:i/>");
    }
    props.push_str(&format!("<w:color w:val=\"{}\"/><w:sz w:val=\"{}\"/><w:szCs w:val=\"{}\"/>", r.colour, r.size, r.size));
    format!("<w:r><w:rPr>{props}</w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r>", esc(&r.text))
}

fn para(inner: &str, before: u32, after: u32, page_break: bool, centred: bool) -> String {
    let mut props = String::new();
    if page_break {
        props.push_str("<w:pageBreakBefore/>");
    }
    props.push_str(&format!("<w:spacing w:before=\"{before}\" w:after=\"{after}\"/>"));
    if centred {
        props.push_str("<w:jc w:val=\"center\"/>");
    }
    format!("<w:p><w:pPr>{props}</w:pPr>{inner}</w:p>")
}

fn drawing_xml(rel: &str, n: usize, p: &Picture) -> String {
    let (cx, cy) = (p.w as u64 * 9525, p.h as u64 * 9525);
    format!(
        "<w:r><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\"><wp:extent cx=\"{cx}\" cy=\"{cy}\"/><wp:docPr id=\"{n}\" name=\"Picture {n}\"/>\
<a:graphic xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
<pic:pic xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><pic:nvPicPr><pic:cNvPr id=\"{n}\" name=\"image{n}.{ext}\"/><pic:cNvPicPr/></pic:nvPicPr>\
<pic:blipFill><a:blip r:embed=\"{rel}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>\
<pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic>\
</a:graphicData></a:graphic></wp:inline></w:drawing></w:r>",
        ext = p.ext
    )
}

/// The bytes of the `.docx` holding `blocks`.
pub fn write(blocks: &[Block]) -> Result<Vec<u8>, String> {
    let mut body = String::new();
    let mut pictures: Vec<(&Picture, String)> = Vec::new();
    for block in blocks {
        match block {
            Block::Text { runs, before, after, page_break } => {
                body.push_str(&para(&runs.iter().map(run_xml).collect::<String>(), *before, *after, *page_break, true));
            }
            Block::Blank { before, after, page_break } => body.push_str(&para("", *before, *after, *page_break, false)),
            Block::Picture { picture, before, after } => {
                let n = pictures.len() + 1;
                let rel = format!("rIdImg{n}");
                body.push_str(&para(&drawing_xml(&rel, n, picture), *before, *after, false, true));
                pictures.push((picture, rel));
            }
        }
    }
    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\"><w:body>{body}\
<w:sectPr><w:pgSz w:w=\"{PAGE_W}\" w:h=\"{PAGE_H}\"/><w:pgMar w:top=\"{MARGIN_TOP}\" w:right=\"{MARGIN_SIDE}\" w:bottom=\"{MARGIN_TOP}\" w:left=\"{MARGIN_SIDE}\" w:header=\"0\" w:footer=\"0\" w:gutter=\"0\"/></w:sectPr></w:body></w:document>"
    );
    let styles = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"Calibri\" w:hAnsi=\"Calibri\" w:cs=\"Calibri\"/><w:sz w:val=\"24\"/><w:szCs w:val=\"24\"/></w:rPr></w:rPrDefault></w:docDefaults></w:styles>";
    let mut doc_rels = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rIdStyles\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>",
    );
    let mut exts: Vec<&str> = Vec::new();
    for (i, (p, rel)) in pictures.iter().enumerate() {
        doc_rels.push_str(&format!("<Relationship Id=\"{rel}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/image{}.{}\"/>", i + 1, p.ext));
        if !exts.contains(&p.ext) {
            exts.push(p.ext);
        }
    }
    doc_rels.push_str("</Relationships>");
    let mut types = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/>",
    );
    for ext in exts {
        let mime = match ext {
            "png" => "image/png",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            _ => "image/jpeg",
        };
        types.push_str(&format!("<Default Extension=\"{ext}\" ContentType=\"{mime}\"/>"));
    }
    types.push_str("<Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/><Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/></Types>");
    let root_rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>";

    let mut zip = ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut put = |name: &str, bytes: &[u8]| -> Result<(), String> {
        zip.start_file(name, opts).map_err(|e| e.to_string())?;
        zip.write_all(bytes).map_err(|e| e.to_string())
    };
    put("[Content_Types].xml", types.as_bytes())?;
    put("_rels/.rels", root_rels.as_bytes())?;
    put("word/document.xml", document.as_bytes())?;
    put("word/styles.xml", styles.as_bytes())?;
    put("word/_rels/document.xml.rels", doc_rels.as_bytes())?;
    for (i, (p, _)) in pictures.iter().enumerate() {
        put(&format!("word/media/image{}.{}", i + 1, p.ext), &p.data)?;
    }
    Ok(zip.finish().map_err(|e| e.to_string())?.into_inner())
}

/// The size of a PNG or JPEG in pixels, read from its header.
pub fn image_size(data: &[u8], png: bool) -> Option<(u32, u32)> {
    if png {
        let w = u32::from_be_bytes(data.get(16..20)?.try_into().ok()?);
        let h = u32::from_be_bytes(data.get(20..24)?.try_into().ok()?);
        return Some((w, h));
    }
    // JPEG: walk the markers up to the start-of-frame
    let mut i = 2;
    while i + 3 < data.len() {
        if data[i] != 0xFF {
            return None;
        }
        let marker = data[i + 1];
        let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            let h = u16::from_be_bytes([*data.get(i + 5)?, *data.get(i + 6)?]) as u32;
            let w = u16::from_be_bytes([*data.get(i + 7)?, *data.get(i + 8)?]) as u32;
            return Some((w, h));
        }
        i += 2 + len;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn a_document_is_a_zip_with_its_parts_and_escaped_text() {
        // a 1 x 1 PNG
        let png: Vec<u8> = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0x0D, 0x49, 0x48, 0x44, 0x52, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 0x1F, 0x15, 0xC4, 0x89,
        ];
        assert_eq!(image_size(&png, true), Some((1, 1)));
        let blocks = vec![
            Block::Picture { picture: Picture { data: png, ext: "png", w: 673, h: 673 }, before: 0, after: 360 },
            Block::Text {
                runs: vec![Run { text: "Rock & <Roll>".to_owned(), size: 48, font: "Calibri Light", bold: true, italic: false, colour: "7c6fff" }],
                before: 0,
                after: 480,
                page_break: false,
            },
            Block::Blank { before: 0, after: 0, page_break: true },
        ];
        let bytes = write(&blocks).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        for name in ["[Content_Types].xml", "_rels/.rels", "word/document.xml", "word/styles.xml", "word/_rels/document.xml.rels", "word/media/image1.png"] {
            assert!(zip.by_name(name).is_ok(), "{name}");
        }
        let mut xml = String::new();
        zip.by_name("word/document.xml").unwrap().read_to_string(&mut xml).unwrap();
        assert!(xml.contains("Rock &amp; &lt;Roll&gt;"));
        assert!(xml.contains("<w:pageBreakBefore/>"));
        assert!(xml.contains("r:embed=\"rIdImg1\""));
    }
}
