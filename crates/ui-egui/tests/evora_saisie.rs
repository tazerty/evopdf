//! Evora (EVORA-COL-63): saving the Colophon Atelier's document while the member types, and the text
//! the Atelier writes, judged in the real shell (egui_kittest) and in the bytes Evora receives. Each
//! test collects every fault before failing and prints its count of checks.
//!
//! - « saisie », « saisie-paragraphe », « saisie-champ » (F4, Evora's D3-e): Evora asks for a save
//!   (`evora:request-save`, any reason: autosave, a hidden tab, the member's button, leaving) while
//!   the member types new text, retypes an existing paragraph, or fills a form field. The bridge's
//!   answer (`PdfCraftApp::host_save_answer`, then `save_for_host`, as `apps/pdfcraft-web` runs them)
//!   hands Evora bytes that hold the whole text typed so far, the box stays open on the keyboard with
//!   the same tool, the next letters (H among them, the Hand's key) go on into the text, and a second
//!   save in the middle carries the longer text. Evora's acknowledgement leaves nothing to save until
//!   the member types again; committing the text afterwards writes it once.
//! - « unicode » (D3-d): added text the standard fonts can't draw (Tahitian, Polish, Russian, Greek,
//!   Vietnamese) is written as real text with Inter Regular embedded, never an image; Chinese, which
//!   Inter doesn't cover, stays refused (witness); Western European text keeps the standard font (twin).
//! - « polices » (Poppler's « xref num … not found but needed »): every font the Atelier's added text
//!   puts in a page's resources is an indirect object.
//!
//! With `EVORA_SAISIE_PDF_DIR` set, the bytes judged here are written there (and, for « unicode », the
//! text expected), so that the CI reads them with Poppler (`pdffonts`, `pdfinfo`, `pdftotext`).

mod evora_support;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use egui_kittest::Harness;
use evora_support::{Verdict, click_page, fixture, harness, harness_with, type_text};
use pdfcraft_cos::Object;
use pdfcraft_engine::{AddedContent, AddedText, Edit, Session};
use pdfcraft_ui_egui::{HostSaveAnswer, PdfCraftApp, QuickTool, SaveTarget};

/// What the member types before Evora's first save, between the two saves (H picks the Hand), after.
const FIRST: &str = "Ligne ajoutée dans Col";
const MIDDLE: &str = "oph";
const LAST: &str = "on";

/// Evora's side of the saves: the bytes of each save handed over, under its ticket.
#[derive(Clone, Default)]
struct Host {
    saves: Arc<Mutex<Vec<(u64, Vec<u8>)>>>,
    next: Arc<AtomicU64>,
}

impl Host {
    /// Installed as the shell's acknowledging host, as the web bridge installs Evora's.
    fn install(&self, app: &mut PdfCraftApp) {
        let host = self.clone();
        app.host_save_ack = Some(Box::new(move |_doc: u64, _name: &str, bytes: &[u8], _save_as: bool| -> Result<Option<u64>, String> {
            let ticket = host.next.fetch_add(1, Ordering::SeqCst) + 1;
            if let Ok(mut saves) = host.saves.lock() {
                saves.push((ticket, bytes.to_vec()));
            }
            Ok(Some(ticket))
        }));
    }

    fn count(&self) -> usize {
        self.saves.lock().map(|s| s.len()).unwrap_or(0)
    }

    fn last(&self) -> Option<(u64, Vec<u8>)> {
        self.saves.lock().ok().and_then(|s| s.last().cloned())
    }
}

/// Evora's `evora:request-save` as the web bridge handles it (`EvoraApp::request_save`): its answer,
/// and the save it then runs.
fn request_save(h: &mut Harness<'static, PdfCraftApp>) -> (HostSaveAnswer, Result<bool, &'static str>) {
    let answer = h.state().host_save_answer(0);
    let saved = if answer == HostSaveAnswer::Save { h.state_mut().save_for_host(0) } else { Ok(false) };
    h.run_steps(3);
    (answer, saved)
}

/// Evora's `evora:saved {ok: true}` for `ticket`, applied on the next frames.
fn acknowledge(h: &mut Harness<'static, PdfCraftApp>, ticket: u64) {
    let outcomes = h.state().host_save_outcomes.clone();
    if let Ok(mut queue) = outcomes.lock() {
        queue.push((ticket, true));
    }
    h.run_steps(3);
}

/// `bytes`, reopened as Evora's next open would.
fn reopened(bytes: &[u8]) -> Option<(Session, pdfcraft_engine::DocId)> {
    let mut session = Session::new();
    let id = session.open("evora.pdf", None, Arc::new(bytes.to_vec()), None).ok()?;
    Some((session, id))
}

/// The lines of text on page 1 of `bytes` and their fonts (`/BaseFont`).
fn page_lines(bytes: &[u8]) -> Vec<(String, String)> {
    let Some((session, id)) = reopened(bytes) else { return Vec::new() };
    session.get(id).map(|d| d.text_lines(0).into_iter().map(|l| (l.text, l.base_font)).collect()).unwrap_or_default()
}

/// The text of page 1 of `bytes`, its lines joined.
fn page_text(bytes: &[u8]) -> String {
    page_lines(bytes).into_iter().map(|(text, _)| text).collect::<Vec<_>>().join("\n")
}

/// The value of form field `name` in `bytes`.
fn field_value(bytes: &[u8], name: &str) -> Option<String> {
    let (session, id) = reopened(bytes)?;
    session.get(id)?.form.iter().find(|f| f.name == name).and_then(|f| f.value.first().cloned())
}

/// The text of the Add text box open on page 1, if any.
fn draft_text(h: &Harness<'static, PdfCraftApp>) -> Option<String> {
    h.state().views.first().and_then(|v| v.content.draft.as_ref()).map(|d| d.text.clone())
}

/// The text of the paragraph open for editing, if any.
fn paragraph_text(h: &Harness<'static, PdfCraftApp>) -> Option<String> {
    h.state().views.first().and_then(|v| v.line_editor.as_ref()).map(|ed| ed.text.clone())
}

/// The form field open for typing and its text, if any.
fn field_typing(h: &Harness<'static, PdfCraftApp>) -> Option<(String, String)> {
    h.state().views.first().and_then(|v| v.forms.focus.as_ref()).map(|f| (f.name.clone(), f.text.clone()))
}

/// The texts added to the document (committed), in order.
fn added_texts(h: &Harness<'static, PdfCraftApp>) -> Vec<String> {
    let app = h.state();
    let Some(doc) = app.views.first().and_then(|v| app.session.get(v.id)) else { return Vec::new() };
    doc.added()
        .iter()
        .filter_map(|a| match &a.content {
            AddedContent::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect()
}

/// `bytes` written as `name` under `EVORA_SAISIE_PDF_DIR`, when the CI sets it.
fn keep(name: &str, bytes: &[u8]) {
    if let Ok(dir) = std::env::var("EVORA_SAISIE_PDF_DIR") {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(std::path::Path::new(&dir).join(name), bytes);
    }
}

/// The last save Evora received: its ticket, and the text of its page 1.
fn last_save(host: &Host, keep_as: &str) -> (Option<u64>, String) {
    match host.last() {
        Some((ticket, bytes)) => {
            keep(keep_as, &bytes);
            (Some(ticket), page_text(&bytes))
        }
        None => (None, String::new()),
    }
}

#[test]
fn every_save_keeps_added_text_typing() {
    let mut v = Verdict::new("saisie");
    let mut h = harness();
    let host = Host::default();
    host.install(h.state_mut());
    let out = std::env::temp_dir().join(format!("evora-saisie-{}.pdf", std::process::id()));
    h.state_mut().save_override = Some(out.to_string_lossy().into_owned());
    let picked = h.state_mut().execute("edit.text");
    h.run_steps(2);
    v.holds(picked && h.state().quick_tool == QuickTool::AddText, || format!("prémisse : outil {:?} après edit.text", h.state().quick_tool));
    click_page(&mut h, 40.0, 110.0);
    type_text(&mut h, FIRST);
    v.holds(draft_text(&h).as_deref() == Some(FIRST), || format!("prémisse : texte en cours {:?} (attendu : {FIRST:?})", draft_text(&h)));
    // Evora's first save (its autosave, say) comes now.
    let (answer, saved) = request_save(&mut h);
    v.holds(answer == HostSaveAnswer::Save && saved == Ok(false), || format!("1er enregistrement : réponse {answer:?}, {saved:?} (attendus : Save, Ok(false) en attente d'Evora)"));
    let (ticket, text) = last_save(&host, "saisie-1.pdf");
    v.holds(text.contains(FIRST), || format!("1er enregistrement : texte de la page {text:?} (attendu : {FIRST:?})"));
    v.holds(draft_text(&h).as_deref() == Some(FIRST), || format!("1er enregistrement : zone de texte {:?} (attendue ouverte : {FIRST:?})", draft_text(&h)));
    v.holds(added_texts(&h).is_empty(), || format!("1er enregistrement : texte validé dans le document {:?}", added_texts(&h)));
    if let Some(t) = ticket {
        acknowledge(&mut h, t);
    }
    v.holds(!h.state().has_unsaved_work(0), || "accusé d'Evora : il reste quelque chose à enregistrer (la saisie l'est)".to_string());
    type_text(&mut h, MIDDLE);
    let middle = format!("{FIRST}{MIDDLE}");
    v.holds(h.state().has_unsaved_work(0), || "frappe après l'accusé : rien à enregistrer".to_string());
    // A second save in the middle (the tab hidden, say).
    let (answer, _) = request_save(&mut h);
    let (_, text) = last_save(&host, "saisie-2.pdf");
    v.holds(answer == HostSaveAnswer::Save && text.contains(&middle), || format!("2e enregistrement : réponse {answer:?}, texte {text:?} (attendu : {middle:?})"));
    type_text(&mut h, LAST);
    let full = format!("{middle}{LAST}");
    let (now, tool) = (draft_text(&h), h.state().quick_tool);
    v.holds(now.as_deref() == Some(full.as_str()), || format!("frappe après les enregistrements : texte {now:?} (attendu : {full:?})"));
    v.holds(tool == QuickTool::AddText, || format!("frappe après les enregistrements : outil {tool:?} (attendu : AddText ; H choisit la Main)"));
    v.holds(host.count() == 2, || format!("enregistrements reçus par Evora : {} (attendus : 2)", host.count()));
    // The member commits and saves: the text is written once.
    let written = h.state_mut().save_view(0, SaveTarget::InPlace);
    h.run_steps(3);
    let added = added_texts(&h);
    v.holds(written && added == [full.clone()], || format!("texte validé puis enregistré : {added:?} (attendu : [{full:?}], écrit {written})"));
    let _ = std::fs::remove_file(&out);
    v.finish();
}

#[test]
fn every_save_keeps_a_paragraph_open() {
    let mut v = Verdict::new("saisie-paragraphe");
    let mut h = harness();
    let host = Host::default();
    host.install(h.state_mut());
    let picked = h.state_mut().execute("edit.edit_text");
    h.run_steps(2);
    v.holds(picked && h.state().quick_tool == QuickTool::EditText, || format!("prémisse : outil {:?} après edit.edit_text", h.state().quick_tool));
    // « Atelier », Helvetica 14, baseline 150 points up a 200-point page: 46 points from the top.
    click_page(&mut h, 30.0, 46.0);
    let opened = paragraph_text(&h);
    v.holds(opened.as_deref().is_some_and(|t| t.contains("Atel")), || format!("prémisse : paragraphe ouvert {opened:?} (attendu : « Atelier »)"));
    type_text(&mut h, "Hamst");
    let typed = paragraph_text(&h).unwrap_or_default();
    v.holds(typed.contains("Hamst"), || format!("prémisse : paragraphe tapé {typed:?}"));
    let (answer, saved) = request_save(&mut h);
    v.holds(answer == HostSaveAnswer::Save && saved == Ok(false), || format!("enregistrement : réponse {answer:?}, {saved:?} (attendus : Save, Ok(false))"));
    let (ticket, text) = last_save(&host, "paragraphe.pdf");
    v.holds(text.contains("Hamst"), || format!("enregistrement : texte de la page {text:?} (attendu : {typed:?})"));
    v.holds(paragraph_text(&h).as_deref() == Some(typed.as_str()), || format!("enregistrement : paragraphe {:?} (attendu ouvert : {typed:?})", paragraph_text(&h)));
    if let Some(t) = ticket {
        acknowledge(&mut h, t);
    }
    type_text(&mut h, "er");
    let now = paragraph_text(&h).unwrap_or_default();
    v.holds(now.contains("Hamster"), || format!("frappe après l'enregistrement : paragraphe {now:?} (attendu : « Hamster »)"));
    v.holds(h.state().quick_tool == QuickTool::EditText, || format!("frappe après l'enregistrement : outil {:?} (attendu : EditText)", h.state().quick_tool));
    v.finish();
}

#[test]
fn every_save_keeps_a_form_field_open() {
    let mut v = Verdict::new("saisie-champ");
    let mut h = harness_with("formulaire.pdf", include_bytes!("data/form.pdf").to_vec());
    let host = Host::default();
    host.install(h.state_mut());
    // The text field « name » (20…280 × 350…372 points on a 300 × 400 page): its middle, from the top.
    click_page(&mut h, 150.0, 39.0);
    let field = field_typing(&h);
    v.holds(field.as_ref().is_some_and(|(name, _)| name == "name"), || format!("prémisse : champ ouvert {field:?} (attendu : « name »)"));
    type_text(&mut h, "Ada");
    let (answer, saved) = request_save(&mut h);
    v.holds(answer == HostSaveAnswer::Save && saved == Ok(false), || format!("enregistrement : réponse {answer:?}, {saved:?} (attendus : Save, Ok(false))"));
    let value = host.last().and_then(|(_, bytes)| {
        keep("champ.pdf", &bytes);
        field_value(&bytes, "name")
    });
    v.holds(value.as_deref() == Some("Ada"), || format!("enregistrement : valeur du champ {value:?} (attendue : « Ada »)"));
    let field = field_typing(&h);
    v.holds(field.as_ref().is_some_and(|(_, text)| text == "Ada"), || format!("enregistrement : champ {field:?} (attendu ouvert : « Ada »)"));
    if let Some((t, _)) = host.last() {
        acknowledge(&mut h, t);
    }
    type_text(&mut h, " Lovelace");
    let field = field_typing(&h);
    v.holds(field.as_ref().is_some_and(|(_, text)| text == "Ada Lovelace"), || format!("frappe après l'enregistrement : champ {field:?} (attendu : « Ada Lovelace »)"));
    v.finish();
}

/// Text item `s` on page 1 of the fixture, in its box.
fn added(s: &str) -> AddedText {
    AddedText { rect: [20.0, 100.0, 290.0, 120.0], text: s.to_string(), size: 14.0, ..Default::default() }
}

/// The fixture with `s` added as text, as the Atelier's Add text commits it, saved.
fn add_and_save(s: &str) -> Result<Vec<u8>, String> {
    let mut session = Session::new();
    let id = session.open("unicode.pdf", None, Arc::new(fixture()), None).map_err(|e| e.to_string())?;
    session.apply(id, Edit::AddText { page: 0, text: added(s) }).map_err(|e| e.to_string())?;
    session.save_bytes(id).map(|b| b.to_vec()).map_err(|e| e.to_string())
}

/// What `bytes` hold: a full font program embedded by `pdfcraft_fonts::embedding` (its streams carry
/// `/PCFullFont`), and the number of image streams.
fn embedded_and_images(bytes: &[u8]) -> (bool, usize) {
    let Ok(doc) = pdfcraft_cos::Document::open(Arc::new(bytes.to_vec())) else { return (false, usize::MAX) };
    let mut embedded = false;
    let mut images = 0;
    for (_, object) in doc.scan_objects() {
        if let Object::Stream(s) = &*object {
            embedded |= s.dict.get(b"PCFullFont").is_some();
            images += usize::from(s.dict.name(b"Subtype") == Some(b"Image"));
        }
    }
    (embedded, images)
}

#[test]
fn added_text_beyond_western_europe_is_real_text() {
    let mut v = Verdict::new("unicode");
    for (i, s) in ["Pāpeʻete Māʻohi", "Zażółć", "Привет", "Ωμέγα", "Tiếng Việt"].into_iter().enumerate() {
        let refused = pdfcraft_engine::first_undrawable(&added(s));
        v.holds(refused.is_none(), || format!("« {s} » : refusé à la saisie (caractère {refused:?})"));
        match add_and_save(s) {
            Ok(bytes) => {
                keep(&format!("unicode-{i}.pdf"), &bytes);
                keep(&format!("unicode-{i}.txt"), s.as_bytes());
                let lines = page_lines(&bytes);
                let inter = lines.iter().any(|(text, font)| text.contains(s) && font.contains("Inter"));
                v.holds(inter, || format!("« {s} » : lignes extraites {lines:?} (attendu : le texte, en Inter)"));
                let (embedded, images) = embedded_and_images(&bytes);
                v.holds(embedded && images == 0, || format!("« {s} » : police incorporée {embedded}, images {images} (attendus : oui, 0)"));
            }
            Err(e) => v.holds(false, || format!("« {s} » : ajout refusé ({e})")),
        }
    }
    // Witness: what Inter doesn't cover stays refused.
    let refused = pdfcraft_engine::first_undrawable(&added("中文"));
    v.holds(refused == Some('中') && add_and_save("中文").is_err(), || format!("« 中文 » : refus attendu (caractère {refused:?})"));
    // Twin: Western European text keeps the standard font, nothing embedded.
    match add_and_save("Café crème") {
        Ok(bytes) => {
            let lines = page_lines(&bytes);
            let standard = lines.iter().any(|(text, font)| text.contains("Café crème") && font.contains("Helvetica"));
            let (embedded, _) = embedded_and_images(&bytes);
            v.holds(standard && !embedded, || format!("« Café crème » : lignes {lines:?}, police incorporée {embedded} (attendus : Helvetica, non)"));
        }
        Err(e) => v.holds(false, || format!("« Café crème » : ajout refusé ({e})")),
    }
    v.finish();
}

/// Every entry of a page's `/Resources /Font` in `bytes`: its name, and whether it is an indirect
/// object (a font written straight into the dictionary sends Poppler looking up a made-up number).
fn page_fonts(bytes: &[u8]) -> Vec<(String, bool)> {
    let Ok(doc) = pdfcraft_cos::Document::open(Arc::new(bytes.to_vec())) else { return Vec::new() };
    let mut out = Vec::new();
    for (_, object) in doc.scan_objects() {
        let Object::Dict(d) = &*object else { continue };
        if d.name(b"Type") != Some(b"Page") {
            continue;
        }
        let fonts = d.get(b"Resources").and_then(|r| doc.dict(r)).and_then(|r| r.get(b"Font").and_then(|f| doc.dict(f)));
        for (name, font) in fonts.iter().flat_map(|f| f.iter()) {
            out.push((String::from_utf8_lossy(name).into_owned(), matches!(font, Object::Ref(_))));
        }
    }
    out
}

#[test]
fn fonts_of_added_text_are_indirect_objects() {
    let mut v = Verdict::new("polices");
    for (i, s) in ["Bonjour", "Привет"].into_iter().enumerate() {
        match add_and_save(s) {
            Ok(bytes) => {
                keep(&format!("polices-{i}.pdf"), &bytes);
                let fonts = page_fonts(&bytes);
                let direct: Vec<&String> = fonts.iter().filter(|(_, indirect)| !indirect).map(|(name, _)| name).collect();
                v.holds(fonts.len() >= 2, || format!("« {s} » : polices de la page {fonts:?} (témoin : F1 et la police ajoutée)"));
                v.holds(direct.is_empty(), || format!("« {s} » : police(s) écrite(s) directement dans /Resources /Font : {direct:?}"));
            }
            Err(e) => v.holds(false, || format!("« {s} » : ajout refusé ({e})")),
        }
    }
    v.finish();
}
