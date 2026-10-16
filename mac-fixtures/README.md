# Mac test fixtures for fredapps-r1

Synthetic files for `../../../mac-checklist-fredapps-r1.md`, built on chaukha with pikepdf (qpdf) and raw bytes written by Python. Copy them to `~/Desktop` on the Mac. Text is Helvetica (`/F1`) unless noted.

| File | Checks | Contents and how it was made | Passwords |
|---|---|---|---|
| `prefixed.pdf` | 9–10 | One text page (pikepdf, classic xref) with 128 `#` bytes put before `%PDF-`. Offsets and `startxref` count from the header, as in #2525's test. Ghostscript warns about the bytes before the header (intended). | – |
| `inline-encrypt.pdf` | 11–12 | One text page, AES-256 (R6) by pikepdf. The `/Encrypt` dictionary was moved inline into the trailer, as MuPDF writes it; the indirect object was blanked and put on the free list. `grep -ac '/Encrypt *<<'` prints 1. | user `user`, owner `owner` |
| `perms.pdf` | 13–15 | One text page, AES-256 (R6) by pikepdf, allowing high-resolution printing only. `/P` was then edited in place to `-4` (padded with spaces); `/Perms` still holds the print-only value. | user `user`, owner `owner` |
| `renumbered.pdf` | 16–17 | Three pages, a lossless RGB picture on page 1. Written in QDF mode by pikepdf, then the first xref subsection header `0 15` was changed to `1 15`, so it starts with object 0's row (#2717). qpdf/pikepdf cannot open it ("unable to find /Root"); that is the case under test. | – |
| `shaded-paragraph.pdf` | 18–19 | A four-row paragraph; before each row's `BT … ET` a yellow box is filled behind that row (`re f`), the structure of #2558's test. | – |
| `slanted.pdf` | 20 | The three rows of #2577's test, drawn with a 30° text matrix (`14.4 TL`, `T*` between rows). | – |
| `vertical-label.pdf` | 21 | One row, "A single turned row", turned 90° (`0 1 -1 0 60 300 Tm`), as in #2710's test, plus an upright paragraph. | – |
| `watermark.pdf` | 23–24 | A nine-row body paragraph (12 pt) and "DRAFT COPY" in 48 pt light grey at 45°, whose origin is the start of body row 4 (same baseline), as in #2590's test. Both are ordinary page content. | – |
| `full-font.pdf` | 25–26 | Two pages of Latin text in DejaVu Sans (`/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf`, Bitstream Vera licence, has Cyrillic), embedded whole as a simple TrueType font (WinAnsi, `/FontFile2`, no subset tag; `pdffonts` shows `sub` = no). Not Arial. The program is 760 KB raw, about 380 KB compressed. | – |
| `wide-page.pdf` | 35 | Five text pages; page 1's MediaBox is `[0 0 6000 600]`, the others Letter. | – |
| `decode-a.pdf` | 48–49 | One page with a 160 × 120 RGB gradient picture (8-bit, Flate) whose dictionary has `/Decode [1 0 0 1 0 1]` (red inverted). | – |
| `decode-b.pdf` | 50 | The same picture with the malformed `/Decode [1 0]`. | – |
| `openaction-js.pdf` | 53 | One text page; the catalog's `/OpenAction` is an indirect `<< /S /JavaScript /JS (var x = 1;) >>`, as in #2687's test. | – |
| `tab-field.pdf` | 64–65 | Two filled text fields with appearances: `First<TAB>Name` = `original, "value"` and `Other` = `other field` (#2688's test values). | – |
| `xfa-pagearea.pdf` | 66 | The `shell` of `crates/xfa/src/fixtures.rs` (`/NeedsRendering true`, one placeholder page, XFA stream) around the XDP of `page_area_without_occur_serves_every_body_page`. | – |
| `xfa-formcalc.pdf` | 67 | The same shell around a one-page XDP with two read-only text fields calculated by FormCalc `Str(2.5, 3, 0)` and `Format("9.99", 0.125)`. | – |
| `indirect-rotate.pdf` | 68 | Three text pages written as raw bytes; page 1 has `/Rotate 10 0 R` and object 10 is `270`. | – |

All files open with pikepdf (with the user password where encrypted) except `renumbered.pdf`, and Ghostscript renders all of them without errors.
