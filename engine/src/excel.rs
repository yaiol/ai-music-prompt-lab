//! The spreadsheet of cards: one workbook with the sheets Card, Project, Song Links and URLs.

use crate::{
    dates,
    store::{Card, Project},
};
use rust_xlsxwriter::{Workbook, Worksheet, XlsxError};

/// A cell: text or a number.
enum Cell {
    Text(String),
    Num(f64),
}

impl From<&str> for Cell {
    fn from(s: &str) -> Self {
        Cell::Text(s.to_owned())
    }
}

impl From<String> for Cell {
    fn from(s: String) -> Self {
        Cell::Text(s)
    }
}

impl From<i64> for Cell {
    fn from(n: i64) -> Self {
        Cell::Num(n as f64)
    }
}

/// A sheet from a header row, the widths of the columns and the rows.
fn sheet(wb: &mut Workbook, name: &str, header: &[&str], widths: &[f64], rows: Vec<Vec<Cell>>) -> Result<(), XlsxError> {
    let mut ws = Worksheet::new();
    ws.set_name(name)?;
    for (c, h) in header.iter().enumerate() {
        ws.write_string(0, c as u16, *h)?;
        if let Some(w) = widths.get(c) {
            ws.set_column_width(c as u16, *w)?;
        }
    }
    for (r, row) in rows.into_iter().enumerate() {
        for (c, cell) in row.into_iter().enumerate() {
            match cell {
                Cell::Text(t) => ws.write_string(r as u32 + 1, c as u16, t)?,
                Cell::Num(n) => ws.write_number(r as u32 + 1, c as u16, n)?,
            };
        }
    }
    wb.push_worksheet(ws);
    Ok(())
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

/// The workbook as the bytes of an `.xlsx` file. `projects` are the ones whose cards are in `cards`.
pub fn workbook(cards: &[Card], projects: &[Project]) -> Result<Vec<u8>, XlsxError> {
    let mut wb = Workbook::new();

    let rows = cards
        .iter()
        .map(|p| {
            vec![
                capital(&p.kind).into(),
                p.name.clone().into(),
                p.version.max(1).into(),
                p.sort_number.map(Cell::from).unwrap_or(Cell::Text(String::new())),
                p.style.clone().into(),
                p.lyrics.clone().into(),
                p.tags.join(", ").into(),
                (if p.favorite { "Yes" } else { "No" }).into(),
                dates::ymd(p.date).into(),
            ]
        })
        .collect();
    sheet(&mut wb, "Card", &["Card Type", "Card Name", "Card Version", "Card Sort", "Style", "Lyrics", "Tags", "Favorite", "Created"], &[12.0, 30.0, 8.0, 8.0, 60.0, 80.0, 25.0, 10.0, 14.0], rows)?;

    let mut rows: Vec<Vec<Cell>> = Vec::new();
    for proj in projects {
        let parent = proj.parent_id.as_ref().and_then(|id| projects.iter().find(|p| &p.id == id));
        let base = |card: [Cell; 3]| {
            let [name, version, kind] = card;
            vec![
                proj.name.clone().into(),
                proj.version.max(1).into(),
                parent.map(|p| p.name.clone()).unwrap_or_default().into(),
                parent.map(|p| Cell::from(p.version.max(1))).unwrap_or(Cell::Text(String::new())),
                dates::ymd(proj.date).into(),
                name,
                version,
                kind,
            ]
        };
        if proj.card_ids.is_empty() {
            rows.push(base([Cell::Text(String::new()), Cell::Text(String::new()), Cell::Text(String::new())]));
        }
        for id in &proj.card_ids {
            rows.push(match cards.iter().find(|c| &c.id == id) {
                Some(c) => base([c.name.clone().into(), c.version.max(1).into(), c.kind.clone().into()]),
                None => base([Cell::Text("(deleted)".to_owned()), Cell::Text(String::new()), Cell::Text(String::new())]),
            });
        }
    }
    sheet(&mut wb, "Project", &["Project Name", "Project Version", "Parent Folder", "Parent Folder Version", "Created", "Card Name", "Card Version", "Card Type"], &[25.0, 8.0, 25.0, 8.0, 14.0, 30.0, 14.0, 12.0], rows)?;

    let mut links: Vec<Vec<Cell>> = Vec::new();
    for song in cards.iter().filter(|c| c.kind == "song") {
        for id in &song.linked_cards {
            let (name, version, kind) = match cards.iter().find(|c| &c.id == id) {
                Some(c) => (c.name.clone(), Cell::from(c.version.max(1)), c.kind.clone()),
                None => ("(deleted)".to_owned(), Cell::Text(String::new()), String::new()),
            };
            links.push(vec![song.name.clone().into(), song.version.max(1).into(), name.into(), version, kind.into()]);
        }
    }
    if !links.is_empty() {
        sheet(&mut wb, "Song Links", &["Song Name", "Song Version", "Card Name", "Card Version", "Card Type"], &[30.0, 14.0, 30.0, 14.0, 12.0], links)?;
    }

    let mut urls: Vec<Vec<Cell>> = Vec::new();
    for p in cards {
        for u in &p.urls {
            urls.push(vec![p.kind.clone().into(), p.name.clone().into(), p.version.max(1).into(), u.label.clone().into(), u.href.clone().into()]);
        }
    }
    if !urls.is_empty() {
        sheet(&mut wb, "URLs", &["Type", "Card Name", "Card Version", "Label", "URL"], &[10.0, 30.0, 14.0, 15.0, 60.0], urls)?;
    }

    wb.save_to_buffer()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Url;
    use std::io::Read;

    #[test]
    fn the_workbook_has_the_sheets_and_the_rows() {
        let song = Card { id: "s".into(), kind: "song".into(), name: "A song".into(), style: "rock".into(), lyrics: "la".into(), tags: vec!["x".into(), "y".into()], linked_cards: vec!["m".into()], urls: vec![Url { id: "u".into(), label: "suno".into(), href: "https://suno.com/song/1".into() }], ..Default::default() };
        let music = Card { id: "m".into(), kind: "music".into(), name: "A style".into(), ..Default::default() };
        let project = Project { id: "p".into(), name: "LP".into(), card_ids: vec!["s".into(), "gone".into()], ..Default::default() };
        let bytes = workbook(&[song, music], &[project]).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut book = String::new();
        zip.by_name("xl/workbook.xml").unwrap().read_to_string(&mut book).unwrap();
        for name in ["Card", "Project", "Song Links", "URLs"] {
            assert!(book.contains(&format!("name=\"{name}\"")), "{name} sheet is missing");
        }
        let mut strings = String::new();
        zip.by_name("xl/sharedStrings.xml").unwrap().read_to_string(&mut strings).unwrap();
        assert!(strings.contains("A song") && strings.contains("(deleted)") && strings.contains("x, y"));
    }

    #[test]
    fn a_book_with_no_links_and_no_addresses_has_only_two_sheets() {
        let bytes = workbook(&[Card { name: "n".into(), kind: "music".into(), ..Default::default() }], &[]).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut book = String::new();
        zip.by_name("xl/workbook.xml").unwrap().read_to_string(&mut book).unwrap();
        assert!(book.contains("name=\"Card\"") && book.contains("name=\"Project\"") && !book.contains("Song Links") && !book.contains("URLs"));
    }
}
