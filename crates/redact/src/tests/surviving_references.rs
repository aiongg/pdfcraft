//! Redacted content must leave the file: no other reference may keep it reachable by a full save.

use super::*;

/// Whether `secret` survives a full save of `doc`, in the file's bytes or in a decoded stream.
fn full_save_holds(doc: &Document, secret: &[u8]) -> bool {
    let bytes = write_full(doc, &SaveOptions::default()).unwrap();
    let found = |data: &[u8]| data.windows(secret.len()).any(|w| w == secret);
    if found(&bytes) {
        return true;
    }
    let saved = Document::open(Arc::new(bytes)).unwrap();
    saved.scan_objects().into_iter().any(|(_, o)| matches!(&*o, Object::Stream(s) if s.decoded().is_ok_and(|d| found(&d))))
}

#[test]
fn a_redacted_field_listed_in_the_calculation_order_leaves_the_saved_file() {
    let mut doc = one_page(b"", "", vec![]);
    pdfcraft_forms::add_field(&mut doc, 0, [10.0, 50.0, 100.0, 70.0], &pdfcraft_forms::NewField::Text { multiline: false }, Some("ssn")).unwrap();
    pdfcraft_forms::set_value(&mut doc, "ssn", &pdfcraft_forms::FieldValue::Text("SSN-078-05-1120".into())).unwrap();
    let field = pdfcraft_forms::fields(&doc).swap_remove(0).obj;
    let root = doc.root().unwrap();
    let af = doc.get(root).as_dict().unwrap().reference(b"AcroForm").unwrap();
    doc.update_dict(af, |d| d.set(b"CO".to_vec(), Object::Array(vec![Object::Ref(field)]))).unwrap();
    mark(&mut doc, 0, &[[0.0, 40.0, 120.0, 80.0]], "");
    assert_eq!(apply(&mut doc, None).unwrap().fields, 1);
    assert!(!full_save_holds(&doc, b"SSN-078-05-1120"), "the field's value is still reachable");
}
