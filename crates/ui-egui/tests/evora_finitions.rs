//! Evora (EVORA-COL-58): two fixes of the Colophon Atelier's daily use, judged in the real shell
//! (egui_kittest, like `tests/evora_masks.rs`). Each test collects every fault before failing and
//! prints its count of checks, which must not change from run to run.
//!
//! - « saisie » (F4): Evora's automatic save (`evora:request-save {reason: "autosave"}`) comes while
//!   the member types added text. The bridge's answer (`PdfCraftApp::host_save_answer`, which
//!   `apps/pdfcraft-web` follows as written below) is `Busy`: nothing is committed, the box stays on
//!   the keyboard, and the letters typed next (H among them, the Hand's key) go to the text, the tool
//!   unchanged. Twins: a save the member asks for (and every other reason) still commits the typing
//!   and saves it; with nothing to save the answer is `Clean`; with nothing typed an autosave saves.
//! - « glyphes » (F5): the main menu open, no text it paints, its submenus' arrows included, has a
//!   character missing from the interface's fonts (egui's arrow U+23F5 had none since E10 took the
//!   emoji faces out, and showed as « ? »). Twin: the menu's submenus are drawn. Run with
//!   `PDFCRAFT_SYSTEM_FONTS=0`, so that no installed face stands in for a missing glyph.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::{HostSaveAnswer, PdfCraftApp, QuickTool, SaveTarget, family_has_char};

/// What the member types before Evora's autosave comes, and after it (H picks the Hand).
const FIRST: &str = "Ligne ajoutée dans Col";
const REST: &str = "ophon";
/// The fixture's page width, in points (its MediaBox).
const PAGE_W: f32 = 300.0;

/// Collects faults, so that a red run names every one of them.
struct Verdict {
    name: &'static str,
    checks: usize,
    faults: Vec<String>,
}

impl Verdict {
    fn new(name: &'static str) -> Self {
        Self { name, checks: 0, faults: Vec::new() }
    }

    fn holds(&mut self, ok: bool, fault: impl FnOnce() -> String) {
        self.checks += 1;
        if !ok {
            self.faults.push(fault());
        }
    }

    fn finish(self) {
        eprintln!("[{}] {} contrôle(s), {} faute(s)", self.name, self.checks, self.faults.len());
        for f in &self.faults {
            eprintln!("[{}] FAUTE {f}", self.name);
        }
        assert!(self.faults.is_empty(), "{} : {} faute(s) sur {} contrôle(s), détail ci-dessus", self.name, self.faults.len(), self.checks);
    }
}

/// One page of 300 × 200 points with one line of text near its top, the rest of the page empty.
fn fixture() -> Vec<u8> {
    let body = "BT /F1 14 Tf 20 150 Td (Atelier) Tj ET";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 300 200] >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
        format!("<< /Length {} >>\nstream\n{body}\nendstream", body.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{o}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1).as_bytes());
    out
}

fn harness() -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 1000.0)).build_eframe(|_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.open_bytes("finitions.pdf", None, fixture()).expect("the fixture opens");
        app
    });
    // Fonts install on frame 1 and apply on frame 2.
    h.run_steps(4);
    h
}

/// A slow click at `(x, y)` points of page 1, from its top left corner, as a member clicks.
fn click_page(h: &mut Harness<'static, PdfCraftApp>, x: f32, y: f32) {
    let Some(r) = h.state().views.first().and_then(|v| v.page_screen_rect(0)) else { return };
    let k = r.width() / PAGE_W;
    let p = r.min + egui::vec2(x * k, y * k);
    h.hover_at(p);
    h.run_steps(1);
    h.drag_at(p);
    h.run_steps(1);
    h.drop_at(p);
    h.run_steps(3);
}

/// `text` typed as a keyboard does: each letter's key goes down, its character comes, the key goes up.
fn type_text(h: &mut Harness<'static, PdfCraftApp>, text: &str) {
    for c in text.chars() {
        let key = egui::Key::from_name(&c.to_ascii_uppercase().to_string());
        if let Some(key) = key {
            h.key_down(key);
        }
        h.event(egui::Event::Text(c.to_string()));
        if let Some(key) = key {
            h.key_up(key);
        }
        h.run_steps(1);
    }
}

/// The text of the box being typed on page 1, if one is open.
fn draft_text(h: &Harness<'static, PdfCraftApp>) -> Option<String> {
    h.state().views.first().and_then(|v| v.content.draft.as_ref()).map(|d| d.text.clone())
}

/// The texts added to the document (committed), in order.
fn added_texts(h: &Harness<'static, PdfCraftApp>) -> Vec<String> {
    let app = h.state();
    let Some(doc) = app.views.first().and_then(|v| app.session.get(v.id)) else { return Vec::new() };
    doc.added()
        .iter()
        .filter_map(|a| match &a.content {
            pdfcraft_engine::AddedContent::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect()
}

/// The bridge's handling of `evora:request-save` (apps/pdfcraft-web, `EvoraApp::request_save`): its
/// answer, and the save it then runs when the answer is `Save` (natively, to the harness's file).
fn request_save(h: &mut Harness<'static, PdfCraftApp>, autosave: bool) -> HostSaveAnswer {
    let answer = h.state().host_save_answer(0, autosave);
    if answer == HostSaveAnswer::Save {
        let _ = h.state_mut().save_view(0, SaveTarget::InPlace);
    }
    h.run_steps(3);
    answer
}

#[test]
fn an_autosave_never_interrupts_typing() {
    let mut v = Verdict::new("saisie");
    let mut h = harness();
    let out = std::env::temp_dir().join(format!("evora-finitions-{}.pdf", std::process::id()));
    h.state_mut().save_override = Some(out.to_string_lossy().into_owned());
    // Add text, a click on the empty part of the page, the first letters (premises).
    let picked = h.state_mut().execute("edit.text");
    h.run_steps(2);
    v.holds(picked && h.state().quick_tool == QuickTool::AddText, || format!("prémisse : outil {:?} après edit.text", h.state().quick_tool));
    click_page(&mut h, 40.0, 110.0);
    type_text(&mut h, FIRST);
    let typed = draft_text(&h);
    v.holds(typed.as_deref() == Some(FIRST), || format!("prémisse : texte en cours {typed:?} (attendu : {FIRST:?})"));
    v.holds(h.state().typing_session(0), || "prémisse : aucune saisie en cours vue par le pont".to_string());
    // Evora's autosave comes now.
    let answer = request_save(&mut h, true);
    v.holds(answer == HostSaveAnswer::Busy, || format!("reprise automatique pendant la saisie : réponse {answer:?} (attendue : Busy)"));
    v.holds(added_texts(&h).is_empty(), || format!("reprise automatique pendant la saisie : texte validé {:?}", added_texts(&h)));
    type_text(&mut h, REST);
    let full = format!("{FIRST}{REST}");
    let (now, tool) = (draft_text(&h), h.state().quick_tool);
    v.holds(now.as_deref() == Some(full.as_str()), || format!("frappe après la reprise : texte en cours {now:?} (attendu : {full:?})"));
    v.holds(tool == QuickTool::AddText, || format!("frappe après la reprise : outil {tool:?} (attendu : AddText ; H choisit la Main)"));
    // Twins: the member's own save (as close, logout and visibility) commits the typing and saves it.
    let answer = request_save(&mut h, false);
    v.holds(answer == HostSaveAnswer::Save, || format!("enregistrement demandé pendant la saisie : réponse {answer:?} (attendue : Save)"));
    let added = added_texts(&h);
    v.holds(added == [full.clone()], || format!("enregistrement demandé : textes ajoutés {added:?} (attendu : [{full:?}])"));
    let written = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    v.holds(written > 0, || "enregistrement demandé : aucun octet écrit".to_string());
    let answer = request_save(&mut h, true);
    v.holds(answer == HostSaveAnswer::Clean, || format!("reprise automatique sans rien à enregistrer : réponse {answer:?} (attendue : Clean)"));
    // Nothing typed, a change: the autosave saves (the guard holds only while the member types).
    let rotated = h.state_mut().execute("page.rotate");
    h.run_steps(3);
    let answer = request_save(&mut h, true);
    v.holds(rotated && answer == HostSaveAnswer::Save, || format!("reprise automatique hors saisie : réponse {answer:?} (attendue : Save ; rotation {rotated})"));
    let _ = std::fs::remove_file(&out);
    v.finish();
}

/// A character drawn without a glyph shows as « ? »; spaces, controls and format characters draw
/// nothing.
fn drawn(c: char) -> bool {
    !(c.is_whitespace() || c.is_control() || matches!(c as u32, 0x00AD | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x206F | 0xFE00..=0xFE0F | 0xFEFF))
}

/// Every character painted on the last frame that no face of its text's families has in its character
/// map (`family_has_char`), once each.
fn missing_glyphs(h: &Harness<'static, PdfCraftApp>) -> Vec<char> {
    fn texts(shape: &egui::Shape, out: &mut Vec<std::sync::Arc<egui::Galley>>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| texts(s, out)),
            egui::Shape::Text(t) => out.push(t.galley.clone()),
            _ => {}
        }
    }
    let mut galleys = Vec::new();
    for clipped in &h.output().shapes {
        texts(&clipped.shape, &mut galleys);
    }
    let defs = h.ctx.fonts(|f| f.definitions().clone());
    let mut missing: Vec<char> = Vec::new();
    for galley in galleys {
        let families: Vec<egui::FontFamily> = galley.job.sections.iter().map(|s| s.format.font_id.family.clone()).collect();
        for c in galley.text().chars() {
            if drawn(c) && !missing.contains(&c) && !families.iter().any(|f| family_has_char(&defs, f, c)) {
                missing.push(c);
            }
        }
    }
    missing
}

#[test]
fn the_main_menu_draws_every_glyph() {
    let mut v = Verdict::new("glyphes");
    // Premise: the interface's fonts alone, as in the web build (no installed face as a fallback).
    let alone = std::env::var("PDFCRAFT_SYSTEM_FONTS").is_ok_and(|value| value == "0");
    v.holds(alone, || "prémisse : PDFCRAFT_SYSTEM_FONTS=0 absent, des polices du système servent de repli".to_string());
    let mut h = harness();
    // Premise: the detector answers (a letter is drawn, a private-use character isn't).
    let defs = h.ctx.fonts(|f| f.definitions().clone());
    let answers = family_has_char(&defs, &egui::FontFamily::Proportional, 'A') && !family_has_char(&defs, &egui::FontFamily::Proportional, '\u{10FFFD}');
    v.holds(answers, || "prémisse : détecteur de glyphes muet (A sans glyphe ou U+10FFFD avec)".to_string());
    let before = missing_glyphs(&h);
    let submenus = |h: &Harness<'static, PdfCraftApp>| ["File ", "Edit ", "Pages ", "View ", "Help "].map(|p| h.query_all_by_label_contains(p).count());
    let closed = submenus(&h);
    if let Some(menu) = h.query_all_by_label("Menu").next() {
        menu.click();
    }
    h.run_steps(3);
    let open = submenus(&h);
    for ((label, a), b) in ["File", "Edit", "Pages", "View", "Help"].iter().zip(closed).zip(open) {
        v.holds(b > a, || format!("menu principal : témoin, sous-menu « {label} » non dessiné ({a} avant, {b} après)"));
    }
    let after = missing_glyphs(&h);
    let new: Vec<String> = after.iter().filter(|c| !before.contains(*c)).map(|c| format!("U+{:04X} « {c} »", *c as u32)).collect();
    eprintln!("[glyphes] glyphes absents avant le menu (relevé) : {before:?}");
    v.holds(new.is_empty(), || format!("menu principal : caractère(s) sans glyphe dans les polices de l'interface : {new:?}"));
    v.finish();
}
