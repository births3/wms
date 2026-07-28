//! Small deterministic text-PDF renderer shared by server-side export workers.

use std::collections::BTreeMap;

use lopdf::{Document, Object, ObjectId};

pub fn render_text_pdf(text: &str) -> Vec<u8> {
    let printable: String = text
        .chars()
        .map(|value| {
            if value.is_ascii_graphic() || value == ' ' {
                value
            } else {
                ' '
            }
        })
        .take(4000)
        .collect();
    let escaped = printable
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)");
    let stream = format!("BT /F1 8 Tf 36 800 Td ({escaped}) Tj ET");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_string(),
        format!("<< /Length {} >>\nstream\n{}\nendstream", stream.len(), stream),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0_usize];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets.into_iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer << /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

/// Temporarily merges complete PDF documents without creating another archive fact.
pub fn merge_pdfs(documents: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    if documents.is_empty() {
        return Err("at least one PDF is required".to_string());
    }
    if documents.len() == 1 {
        return Ok(documents[0].clone());
    }
    let mut parsed = documents
        .iter()
        .map(|bytes| Document::load_mem(bytes).map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let mut max_id = 1;
    let mut pages = BTreeMap::new();
    let mut objects = BTreeMap::new();
    for document in &mut parsed {
        document.renumber_objects_with(max_id);
        max_id = document.max_id + 1;
        for object_id in document.get_pages().into_values() {
            let object = document
                .get_object(object_id)
                .map_err(|error| error.to_string())?
                .to_owned();
            pages.insert(object_id, object);
        }
        objects.append(&mut document.objects);
    }
    let mut merged = Document::with_version("1.5");
    let mut catalog: Option<(ObjectId, Object)> = None;
    let mut page_tree: Option<(ObjectId, Object)> = None;
    for (object_id, object) in objects {
        match object.type_name().unwrap_or(b"") {
            b"Catalog" if catalog.is_none() => catalog = Some((object_id, object)),
            b"Pages" if page_tree.is_none() => page_tree = Some((object_id, object)),
            b"Catalog" | b"Pages" | b"Page" | b"Outlines" | b"Outline" => {}
            _ => {
                merged.objects.insert(object_id, object);
            }
        }
    }
    let (page_tree_id, page_tree_object) =
        page_tree.ok_or_else(|| "PDF pages root not found".to_string())?;
    for (object_id, object) in &pages {
        let mut dictionary = object.as_dict().map_err(|error| error.to_string())?.clone();
        dictionary.set("Parent", page_tree_id);
        merged
            .objects
            .insert(*object_id, Object::Dictionary(dictionary));
    }
    let mut page_tree_dictionary = page_tree_object
        .as_dict()
        .map_err(|error| error.to_string())?
        .clone();
    page_tree_dictionary.set("Count", pages.len() as u32);
    page_tree_dictionary.set(
        "Kids",
        pages.into_keys().map(Object::Reference).collect::<Vec<_>>(),
    );
    merged
        .objects
        .insert(page_tree_id, Object::Dictionary(page_tree_dictionary));
    let (catalog_id, catalog_object) =
        catalog.ok_or_else(|| "PDF catalog not found".to_string())?;
    let mut catalog_dictionary = catalog_object
        .as_dict()
        .map_err(|error| error.to_string())?
        .clone();
    catalog_dictionary.set("Pages", page_tree_id);
    catalog_dictionary.remove(b"Outlines");
    merged
        .objects
        .insert(catalog_id, Object::Dictionary(catalog_dictionary));
    merged.trailer.set("Root", catalog_id);
    merged.max_id = merged
        .objects
        .keys()
        .map(|object_id| object_id.0)
        .max()
        .unwrap_or(0);
    merged.renumber_objects();
    let mut bytes = Vec::new();
    merged
        .save_to(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendered_pdf_has_pdf_header_and_eof() {
        let pdf = render_text_pdf("WMS PDF");
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
    }

    #[test]
    fn merge_keeps_every_page() {
        let merged = merge_pdfs(&[render_text_pdf("first"), render_text_pdf("second")])
            .expect("PDFs should merge");
        let document = Document::load_mem(&merged).expect("merged PDF should parse");
        assert_eq!(document.get_pages().len(), 2);
    }
}
