# PdfCraft Mac test — `next` ed1ab63cd

Report results by number ("1–9 pass, 14 skipped, 21 fails: …"). Test the first section first: those are the changes most likely to lose work or crash.

## 1. Build (give this to the Mac agent)

```
In ~/aiongg/pdfcraft, build the PdfCraft test app from the fork's `next` branch. Don't launch the app.
1. git remote get-url aiongg >/dev/null 2>&1 || git remote add aiongg https://github.com/aiongg/pdfcraft.git
2. git fetch aiongg next && git checkout -B next aiongg/next && git reset --hard aiongg/next; confirm HEAD is ed1ab63cd.
3. packaging/macos/package.sh --arch aarch64
   (if ~/aiongg/craft-fonts exists, run it as CRAFT_FONTS_DIR=$HOME/aiongg/craft-fonts packaging/macos/package.sh --arch aarch64)
4. mkdir -p ~/aiongg/pdfcraft-test && /bin/cp -Rf target/macos-package/PdfCraft.app ~/aiongg/pdfcraft-test/
5. /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f ~/aiongg/pdfcraft-test/PdfCraft.app
6. Copy the test PDFs to the Desktop: git fetch aiongg mac-fixtures-fredapps-r1 && git archive FETCH_HEAD mac-fixtures | tar -x -C ~/Desktop --strip-components=1 --exclude '*README.md'; confirm FETCH_HEAD is eb43439d3 and 17 PDFs arrived.
Report the commit you built and whether each step succeeded.
```

Then open `~/aiongg/pdfcraft-test/PdfCraft.app`.

## 2. Checks

Items 9–21, 23–26, 35, 48–50, 53 and 64–68 use a prepared file that the build step put on the Desktop (list under Fixtures). "⌘K › Name" means: press ⌘K, type the command name, press Return. Some checks use Terminal and qpdf (`brew install qpdf`).

### Saving over existing files — #2544, #2699, #2757
- [ ] 1. In Finder, duplicate any PDF to the Desktop as `private.pdf`. In Terminal run `chmod 600 ~/Desktop/private.pdf`. Open it, add a sticky note, press ⌘S. `ls -l ~/Desktop/private.pdf`: still `-rw-------`.
- [ ] 2. Run `chmod 640 ~/Desktop/private.pdf`, add another note, ⌘S: `ls -l` still shows `-rw-r-----`.
- [ ] 3. ⇧⌘S the same document to a new name on the Desktop: the new file shows `-rw-r--r--`.
- [ ] 4. Duplicate a PDF to the Desktop as `ro.pdf` and run `chmod 444 ~/Desktop/ro.pdf`. Open it, add a sticky note, press ⌘S: a message says "Couldn't save ro.pdf: the file is read-only; save a copy with Save As", and the tab still shows unsaved changes.
- [ ] 5. `ls -l ~/Desktop/ro.pdf` still shows `-r--r--r--`, and `ro.pdf` opened in Preview has no sticky note.
- [ ] 6. In the same tab, ⇧⌘S to `ro-copy.pdf`: it saves, and the copy has the note.
- [ ] 7. In Apple Mail, open an email with a PDF attachment (send one to yourself). Right-click the attachment › Open With › PdfCraft. Add a sticky note and press ⌘S: a save dialog asks where to keep the file; no error message.
- [ ] 8. Save it to the Desktop: the Desktop copy has the note. A PDF opened from the Desktop still saves in place with ⌘S, with no dialog.

### Saving and security in unusual files — #2525, #2702, #2580, #2717
- [ ] 9. Open `prefixed.pdf` (needs fixture: bytes before the `%PDF-` header). Add a sticky note, ⌘S, close the tab (⌘W) and open the file again: the note is there.
- [ ] 10. In Terminal run `qpdf --check ~/Desktop/prefixed.pdf`: no "xref not found" and no "reconstructing" warning. Preview opens it and shows the note.
- [ ] 11. Open `inline-encrypt.pdf` (needs fixture: password-protected PDF with /Encrypt inline in the trailer) with the password `owner`. ⌘K › Remove security (confirm if asked): the tab shows unsaved changes. Press ⌘S.
- [ ] 12. Close and reopen `inline-encrypt.pdf`: no password prompt, and ⌘D › Security shows "No security". Preview opens it without a password too.
- [ ] 13. Open `perms.pdf` (needs fixture: AES-256 PDF whose /P was edited to allow everything) with the password `user`. ⌘D › Security: Printing shows "High resolution"; Changing the document, Commenting, Filling of form fields and Content copying show "Not allowed".
- [ ] 14. In the same tab, ⌘K › Edit text & images: it can't be used (greyed out, or refused with a message).
- [ ] 15. Close it and open `perms.pdf` with the password `owner`: ⌘D › Security shows "Owner password (no restrictions apply)", and Edit text & images works.
- [ ] 16. Open `renumbered.pdf` (needs fixture: xref subsection headed `1 N` that starts with object 0's row): every page draws, and the picture on page 1 shows.
- [ ] 17. ⇧⌘S it to `renumbered-saved.pdf` and open that file: page 1 still has its picture.

### Editing existing text — #2558, #2577, #2710, #2590, #2594
Use ⌘K › Edit text & images. Click into a paragraph to edit it; click outside it to apply.
- [ ] 18. `shaded-paragraph.pdf` (needs fixture: a paragraph with a coloured background behind each line). Add a few words to its first line and apply: the text of every line shows on top of its background; no line is hidden behind a box.
- [ ] 19. Add enough words that the paragraph gets one more line: the new line also shows above the backgrounds.
- [ ] 20. `slanted.pdf` (needs fixture: a three-line paragraph rotated 30°). Add a sentence in the middle and apply: the text rewraps along the slant, no row is longer than the original rows, and words that don't fit move to the next row.
- [ ] 21. `vertical-label.pdf` (needs fixture: a one-line label rotated 90°). Add about 30 words to the label and apply: it wraps into more rows that stay on the page instead of running off its edge.
- [ ] 22. Any PDF with an upright one-line heading: add many words and apply: it grows to the right margin, then wraps, as before.
- [ ] 23. `watermark.pdf` (needs fixture: a large watermark text that starts on a body line). Change a word in the body line under the watermark and apply: the watermark stays.
- [ ] 24. Delete all the text of that body line and apply: the watermark still stays.
- [ ] 25. `full-font.pdf` (DejaVu Sans embedded in full, not as a subset). Note ⌘D › Advanced › File size. Add `быстрая лиса` to the end of a paragraph on page 1, ⇧⌘S to a new name, reopen it: the file grew by roughly half the font's size (about 380 KB; the font written uncompressed would add about 760 KB).
- [ ] 26. In that copy, add `быстрая лиса` to a paragraph on page 2, ⌘S, reopen: the file grew by only a few KB, and both pages show the Cyrillic words.

### Replace text comments — #2681
- [ ] 27. Any PDF with text: drag over two words with the Select tool, click Replace text in the action bar, type `new words` and confirm: the Comments panel shows one Replace text comment.
- [ ] 28. ⌘K › Export all comments to data file…, save as `.xfdf`. Delete the comment, then ⌘K › Import comments… and pick that file: the Replace text comment comes back as one comment with `new words`, and the old words are struck through.

### Remove hidden information — #2532
- [ ] 29. A tagged PDF with web links: in Chrome, print a Wikipedia article › Save as PDF. Open it, ⌘K › Remove hidden information…, keep links and comments ticked, run it, ⇧⌘S as `cleaned.pdf`. Click where a link was: nothing opens.
- [ ] 30. In Terminal run `qpdf --qdf --object-streams=disable ~/Desktop/cleaned.pdf - | grep -ac /URI`: it prints 0.

### Page turning and Go to page — #2587, #2595, #2703, #2704
- [ ] 31. Any PDF with 10 or more pages. Menu (top left) › View › Single page, untick Continuous scrolling, press ⌘0. Turn the mouse wheel one notch down (or make a short two-finger swipe): the next page appears at once, with no blank frame and no empty white page.
- [ ] 32. Turn quickly through ten pages this way: each page is drawn as it appears.
- [ ] 33. Same view: ⇧⌘N, type 5, Return: page 5 appears in its final place at once, with no small hop afterwards.
- [ ] 34. Turn pages with the arrow keys: each page lands in place without a hop.
- [ ] 35. `wide-page.pdf` (needs fixture: page 1 very wide, the other pages normal). Menu › View › Actual size. ⇧⌘N, type 3, Return: page 3 comes into view, centred, and the counter shows 3.
- [ ] 36. Any PDF zoomed to 400%, so a page is wider than the window. Scroll to the right half of a page, then ⇧⌘N to the next page: the view stays on the same right half.
- [ ] 37. ⌘F, type a word that appears several times. With the cursor still in the find field, press ⌘G: the next match is highlighted. ⇧⌘G: the previous one.

### Page Range Options — #2642
- [ ] 38. A PDF with 3 or more pages: ⌘K › Add background…. Under Page Range Options choose Pages from 1 to 1: "to" stays at 1. Click OK: only page 1 gets the background.
- [ ] 39. Same in Add header & footer… and in Add watermark…: Pages from 1 to 1 stays at 1, and only page 1 gets the mark.

### Selection action bar — #2638
- [ ] 40. With the Select tool, drag over a few words: the action bar appears (Add comment, Highlight, …, Copy). Click Copy: the bar closes and the words stay selected. ⌘V in TextEdit pastes them.
- [ ] 41. Drag over the same words again: the action bar appears again.

### Read mode — #2809
- [ ] 42. In All tools, press ⌃⌘H: Read mode opens and the floating quick toolbar (Select, Hand, …) is gone. ⌃⌘H again brings it back.
- [ ] 43. Choose Hand in the quick toolbar, press ⌃⌘H twice: Select is the active tool.
- [ ] 44. ⌘, › Documents and view › Default workspace mode: Read. Back in All tools, choose Hand, then open another PDF with ⌘O: it opens in Read mode with no quick toolbar. ⌃⌘H: Select is the active tool. Set the preference back to All tools.
- [ ] 45. ⌘K › Edit text & images, click into a line and change a word. Without clicking outside, press ⌃⌘H: Read mode shows the changed word and no edit box. ⌃⌘H again: the change is still there and the tab shows unsaved changes.

### Pictures — #2810, #2793
- [ ] 46. A PDF with a picture: with the Select tool, click the picture: a blue highlight appears. ⌘K › Edit text & images, click the picture, press Delete: the picture and the blue highlight are both gone at once.
- [ ] 47. ⌘Z: the picture comes back. ⇧⌘Z: it goes again, and no highlight is left.
- [ ] 48. `decode-a.pdf` (needs fixture: an RGB picture with `/Decode [1 0 0 1 0 1]`). ⌘K › Edit text & images, right-click the picture › Save Image As…: the saved image, opened in Preview, has the same colours as the page in PdfCraft.
- [ ] 49. Same file: Menu › File › Export all images…: the exported image has the same colours as the page.
- [ ] 50. `decode-b.pdf` (needs fixture: an RGB picture with the malformed `/Decode [1 0]`). Save Image As… saves without an error, and Export all images… includes the picture.

### Document Properties — #2794, #2687
- [ ] 51. Any PDF: add a sticky note and press ⌘S, 15 times. ⌘D › Advanced: the View revision buttons wrap onto several rows, and none covers Binding or the rows below. Make the window narrow: still no overlap.
- [ ] 52. Click View revision 1: it opens in a new tab that says "You are viewing revision 1 of …".
- [ ] 53. `openaction-js.pdf` (needs fixture: a catalog /OpenAction that runs JavaScript). ⌘D › Advanced: JavaScript says Yes. Any plain PDF says No.

### Window size — #2730
- [ ] 54. Resize the window to about half the screen. Minimize it (⌘M), then right-click the Dock icon › Quit. Open PdfCraft again: the window has the size it had, and is not small in a corner.

### Export and create — #2816, #2754, #2679
- [ ] 55. Save a Wikipedia article from Safari (File › Export as PDF…) and open it. Menu › File › Export to HTML…, open the result in Safari: the paragraphs with links read in full as running text, not as a table, and no words are missing.
- [ ] 56. Same with Export to Word… (open in Pages) and Export to PowerPoint… (open in Keynote): the sentences with links are complete.
- [ ] 57. In TextEdit make a plain-text file (Format › Make Plain Text) with one line indented by four spaces and one by a tab; save it as `indent.txt`. ⌘K › Create PDF from file…, pick it: both indents show in the PDF.
- [ ] 58. ⌘K › Add a text box, type four spaces and then `indented`, click outside: the text keeps its indent.
- [ ] 59. ⌘K › Add or edit links, drag a box, enter the web address `https://example.com/ação`, OK. ⌘S, close, reopen, click the link: the browser opens `example.com/ação` (the address bar may show `a%C3%A7%C3%A3o`).
- [ ] 60. Open the saved file in Preview and click the link: the browser opens the same address.

### Form fields — #2602, #2686, #2688, #1917
- [ ] 61. Any PDF: ⌘K › Add a text field and place it. In its Field properties… › Format choose Date and the custom format `yyyy dd mmm HH:MM`. Leave form editing, type `2026 05 Sep` in the field and press Tab: it shows `2026 05 Sep 00:00`, not 12:00.
- [ ] 62. Type `2026 05 Sep 5:30p` and press Tab: it shows `2026 05 Sep 17:30`.
- [ ] 63. A PDF with a filled text field: ⌘K › Export form data…, save as XML. In TextEdit change that field's value to `left<!-- note -->right` and save. ⌘K › Import form data…, pick the file: the field shows `leftright`.
- [ ] 64. `tab-field.pdf` (needs fixture: a form field whose name contains a tab). ⌘K › Export form data…, save as CSV. Change both field values in the app, then ⌘K › Import form data… and pick the CSV: both fields get their exported values back.
- [ ] 65. Same file, exported and imported as XFDF: both fields get their exported values back.

### XFA forms — #2604, #2678
- [ ] 66. `xfa-pagearea.pdf` (needs fixture: XFA form whose first page area has no `<occur>`): it shows four pages, each with one text field, and no Print button on any page.
- [ ] 67. `xfa-formcalc.pdf` (needs fixture: XFA form with FormCalc `Str` and `Format` calculations): the first field shows 3 (not 2), the second 0.13 (not 0.12).

### Page rotation — #1919
- [ ] 68. `indirect-rotate.pdf` (needs fixture: page 1's `/Rotate 270` stored as a separate object): page 1 shows turned. With page 1 current, Menu › Pages › Rotate pages clockwise: it shows upright. ⇧⌘S, reopen: still upright.

Not in this list (no visible effect on the Mac app): #2267 (engine only), #2600 (automation split only), #2601 (web only), #2606 (Windows installer), #2625 (Windows only; Reveal in Finder is unchanged), #2698 (the redaction audit is CLI and automation only).

## Fixtures

The files come prebuilt (the Mac agent copies them to the Desktop; see the build block): `prefixed.pdf`, `inline-encrypt.pdf`, `perms.pdf`, `renumbered.pdf`, `shaded-paragraph.pdf`, `slanted.pdf`, `vertical-label.pdf`, `watermark.pdf`, `full-font.pdf`, `wide-page.pdf`, `decode-a.pdf`, `decode-b.pdf`, `openaction-js.pdf`, `tab-field.pdf`, `xfa-pagearea.pdf`, `xfa-formcalc.pdf`, `indirect-rotate.pdf`. Passwords and contents: `mac-fixtures/README.md`.
