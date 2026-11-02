//! Evora (EVORA-COL-63, D3-d): added text that the standard fonts can't draw, written with Inter
//! Regular, the interface's own face (SIL Open Font License, `assets/fonts`), embedded through
//! `pdfcraft_fonts::embedding` as a Type 0 font (Identity-H) with its ToUnicode map: real text that
//! copies, searches and reads back, never a picture. Tahitian (ā ē ī ō ū ʻ), Polish, Turkish,
//! Cyrillic, Greek and Vietnamese text was refused before; what Inter doesn't cover (Chinese,
//! Japanese, Korean, Indic and Thai scripts…) still is, by [`super::first_undrawable`].
//!
//! The item keeps its parameters (`/PCAdded`), so it is redrawn the same way when it is moved or
//! retyped. Inter has no bold, italic, serif or monospaced face here: bold is drawn with a stroke of
//! the fill's colour and italic slanted, as viewers do for a font without those faces; the family is
//! Inter's. Justified lines are not stretched: word spacing (`Tw`) applies to single-byte codes only.

use super::*;

/// The slant of the italic stand-in (the tangent of about 11 degrees).
const SLANT: f64 = 0.2;
/// The stroke width of the bold stand-in, in ems.
const BOLD_STROKE: f64 = 0.03;

/// Whether text item `t` is written with Inter: it has no Arabic, Hebrew or Thai (their craft-fonts
/// faces draw them), something outside WinAnsiEncoding, and Inter has every one of its characters.
pub(super) fn in_inter(t: &AddedText) -> bool {
    !t.text.chars().any(|c| is_thai(c) || is_arabic(c) || is_hebrew(c))
        && t.text.split('\n').any(|line| pdfcraft_fonts::first_non_win_ansi(line).is_some())
        && pdfcraft_fonts::embedding::covers(pdfcraft_fonts::INTER_REGULAR, &t.text)
}

/// Draw text item `t` ([`in_inter`]) with Inter into `out`, its font resource into `fonts`. `taken`,
/// the page's fonts, gives the resource a free name, or Inter's own when the page has it already.
pub(super) fn draw(doc: &mut Document, t: &AddedText, taken: &Dict, fonts: &mut Dict, out: &mut Vec<u8>) -> Result<(), EditError> {
    let mut page_fonts = taken.clone();
    let name = pdfcraft_fonts::embedding::embed(doc, &mut page_fonts, pdfcraft_fonts::INTER_REGULAR, "Inter").map_err(EditError::Invalid)?;
    let font = page_fonts.get(name.as_bytes()).cloned().ok_or_else(|| EditError::Invalid("the Inter font could not be embedded".into()))?;
    let dict = doc.dict(&font).ok_or_else(|| EditError::Invalid("the embedded Inter font could not be read back".into()))?;
    let metrics = pdfcraft_fonts::pdf::Metrics::from_dict(doc, &dict);
    fonts.set(name.clone().into_bytes(), font);
    let r = text_rect(t);
    let [cr, cg, cb] = t.color.map(|v| v.clamp(0.0, 1.0));
    out.extend(format!("BT /{name} {} Tf {} {} {} rg {} {} {} RG\n", n(t.size), n(cr), n(cg), n(cb), n(cr), n(cg), n(cb)).bytes());
    if t.bold {
        out.extend(format!("2 Tr {} w\n", n(t.size * BOLD_STROKE)).bytes());
    }
    if t.char_spacing != 0.0 {
        out.extend(format!("{} Tc\n", n(t.char_spacing)).bytes());
    }
    let slant = if t.italic { SLANT } else { 0.0 };
    let all = lines(t);
    for (i, line) in all.iter().enumerate() {
        let (x, y, _) = line_origin(t, r, &all, i);
        let code = metrics.encode(line).ok_or_else(|| EditError::Invalid("the Inter font can't encode this text".into()))?;
        out.extend(format!("1 0 {} 1 {} {} Tm ", n(slant), n(x), n(y)).bytes());
        out.extend(literal(&code));
        out.extend_from_slice(b" Tj\n");
    }
    out.extend_from_slice(b"ET\n");
    Ok(())
}
