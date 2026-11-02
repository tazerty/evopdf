//! Evora (patches E7, COL-05, COL-10): what the Colophon Atelier build, served inside Evora, does not
//! offer, and the guard that keeps it out of reach.
//!
//! - E7: the app's own outbound links (the upstream source repository) and the update check (which,
//!   on the web, opens the upstream releases page). Updates of the Atelier are Evora's (pinned
//!   upstream tag, rebuilt by the fork's CI).
//! - COL-05 (Evora decision D-9): digital signatures (PAdES), certification and the creation of
//!   self-signed digital IDs, which happens in the signing dialog only, belong to Evora Sign. The
//!   Signatures panel still shows and validates signatures (read only); Fill & Sign stays.
//! - COL-10 (D-9): redaction is done by Colophon's server tool, which really removes the content,
//!   until a server-side check of redacted areas exists.
//! - OCR: the engine recognizes unaccented Latin script only; Colophon's server OCR tool does the
//!   job, and the guided actions that recognize text (Action Wizard steps) go with it.
//! - Q47 (a), Evora's default hypothesis until it is decided: the Atelier does fine editing only.
//!   The upstream automations, the engine's own recent files, XFDF and FDF imports, the JavaScript
//!   console, the rest of the redaction family (sanitize, remove hidden information) and the rest of
//!   the Scan & OCR family (camera images) are hidden too.
//! - EVORA-COL-53: editing existing text and images (Edit text & images) is offered. Gabriel's order
//!   of 2026-11-01 (« rendre les PDF entièrement éditables ») decides this part of Q47: its condition
//!   is met, Evora Drive keeps versions (F7, lot DRV-4) and Colophon's workspace saves documents as
//!   new Drive versions. The picture menu of that tool keeps its redaction and OCR items out
//!   (`edit_text_ui::image_menu`), and the Edit panel links no hidden tool (`panels::edit_panel`).
//! - EVORA-COL-58 (F6): the AI assistant tool (a model the member would bring himself: Evora's AI
//!   goes through Evora's own service, repository rule 6) and the rest of the « Use JavaScript » tool
//!   (document scripts and actions, whose added scripts Colophon refuses on save) are hidden; both
//!   tool groups disappear. Add a digital signature field (an empty field, no signature) stays.
//! - EVORA-COL-63 (Evora's D3-a): Tools ▸ Document JavaScripts… is hidden too: a script added there
//!   is refused when the document is saved (Colophon's `active_content_added`), as for F6. (D3-b) The
//!   « Use a certificate » tile no longer promises a certification: the catalogue names it after what
//!   it still offers, « Validate all signatures » (read only, D-9).
//!
//! Fixes of the bridge's daily use live here too: no save asked by Evora, whatever its reason, nor
//! the member's own, interrupts typing: the bytes carry the typing and the text box stays open
//! ([`PdfCraftApp::save_for_host`], EVORA-COL-63, F4); a submenu's arrow is drawn in a glyph the
//! interface's fonts have ([`menu_button`], EVORA-COL-58, F5).
//!
//! Hidden commands and catalogue items stay out of the menus, the command palette, the keyboard
//! shortcuts list, the tool panels and Home, and do nothing if run another way; a tool group whose
//! items are all hidden disappears. Every frame, [`enforce`] closes a dialog, drops a page tool and
//! leaves a tool panel that only a hidden command opens, whatever opened it (a click on an empty
//! signature field, the Signatures panel, automation). Links found in documents are not affected:
//! they keep their confirmation dialog.

use pdfcraft_engine::catalog::{self, ToolGroup};
use pdfcraft_engine::{DocId, Edit};

use crate::{Dialog, LeftPanel, PdfCraftApp, QuickTool, SaveTarget};

/// Registry commands and catalogue items hidden from every list and ignored when run.
pub(crate) const HIDDEN_COMMANDS: &[&str] = &[
    // E7.
    "help.github",
    "help.check_updates",
    // COL-05 (D-9): digital signatures, certification, the digital IDs created for them, and the
    // timestamp the catalogue plans beside them.
    "sign.digital",
    "sign.certify",
    "sign.certify_invisible",
    "sign.timestamp",
    // COL-10 (D-9): redaction.
    "redact.mark",
    "redact.pages",
    "redact.search",
    "redact.properties",
    "redact.apply",
    "redact.clear",
    // OCR, planned OCR items, and the guided actions that recognize text.
    "ocr.recognize",
    "ocr.recognize_batch",
    "ocr.enhance",
    "ocr.correct",
    "actions.wizard",
    "actions.optimize_scans",
    // Q47 (a), default hypothesis: upstream automations.
    "actions.distribution",
    "actions.make_accessible",
    "actions.archive",
    // Q47 (a): the engine's recent files (Evora Drive keeps the documents).
    "file.open_recent",
    "file.clear_recent",
    "file.pin_folder",
    // Q47 (a): XFDF and FDF imports.
    "comment.import",
    "form.import_data",
    // Q47 (a): the JavaScript console (document scripts and scripted forms stay).
    "tools.js_console",
    "js.console",
    // Q47 (a): the rest of the redaction family.
    "redact.sanitize",
    "protect.remove_hidden",
    // Q47 (a): the rest of the Scan & OCR family.
    "ocr.enhance_camera",
    // EVORA-COL-58 (F6): the AI assistant, outside Evora's AI service (repository rule 6).
    "ai.summary",
    "ai.ask",
    "ai.translate",
    // EVORA-COL-58 (F6): the rest of the « Use JavaScript » tool (its console is hidden above).
    "js.document",
    "js.actions",
    // TÉMOIN SABORDÉ (EVORA-COL-66, S4) : Tools ▸ Document JavaScripts… n'est plus masqué.
    // EVORA-COL-53: "edit.edit_text" (editing existing text and images) is no longer hidden. Gabriel's
    // order of 2026-11-01 decides Q47 for it, its condition (Evora Drive keeps versions, F7) being met.
];

pub(crate) fn hidden(id: &str) -> bool {
    HIDDEN_COMMANDS.contains(&id)
}

/// A tool group with items, all of them hidden.
pub(crate) fn hidden_group(group: &ToolGroup) -> bool {
    group.sections.iter().any(|s| !s.items.is_empty()) && group.sections.iter().flat_map(|s| s.items).all(|i| hidden(i.command))
}

/// The catalogue's tool groups still on offer, in catalogue order.
pub(crate) fn visible_groups() -> impl Iterator<Item = &'static ToolGroup> {
    catalog::TOOL_GROUPS.iter().filter(|g| !hidden_group(g))
}

/// The tool groups the command palette lists: [`visible_groups`]. Under the CI witness
/// `evora-witness-palette-groups` (never in the web build), every catalogue group, as upstream, so
/// that tests/evora_masks.rs shows its palette checks of hidden groups fail without this filter.
pub(crate) fn palette_groups() -> impl Iterator<Item = &'static ToolGroup> {
    catalog::TOOL_GROUPS.iter().filter(|g| cfg!(feature = "evora-witness-palette-groups") || !hidden_group(g))
}

/// Whether tool panel `id` may open (an id the catalogue doesn't know is left to the caller).
pub(crate) fn tool_allowed(id: &str) -> bool {
    catalog::group(id).is_none_or(|g| !hidden_group(g))
}

/// The command a dialog belongs to, when only that command's family opens it.
fn dialog_command(dialog: &Dialog) -> Option<&'static str> {
    Some(match dialog {
        // Digitally sign, Certify, a click on an empty signature field, the Signatures panel.
        Dialog::Sign => "sign.digital",
        Dialog::RedactPages => "redact.pages",
        Dialog::RedactSearch => "redact.search",
        Dialog::RedactProps => "redact.properties",
        Dialog::RedactApply => "redact.apply",
        Dialog::RecognizeText => "ocr.recognize",
        // The Action Wizard runs and edits actions whose steps recognize text.
        Dialog::ActionWizard => "actions.wizard",
        Dialog::JsConsole => "tools.js_console",
        Dialog::DocumentJs => "tools.document_js",
        Dialog::Sanitize => "redact.sanitize",
        Dialog::RemoveHidden => "protect.remove_hidden",
        _ => return None,
    })
}

/// The command a page tool belongs to.
fn tool_command(tool: &QuickTool) -> Option<&'static str> {
    Some(match tool {
        QuickTool::SignArea { certify: false } => "sign.digital",
        QuickTool::SignArea { certify: true } => "sign.certify",
        QuickTool::Redact => "redact.mark",
        QuickTool::EnhanceCamera => "ocr.enhance_camera",
        // Inert while "edit.edit_text" is offered (EVORA-COL-53); kept so that listing the id again in
        // HIDDEN_COMMANDS hides the tool whole, as the CI witness of that lot does.
        QuickTool::EditText => "edit.edit_text",
        _ => return None,
    })
}

/// Run at the start of every frame (`logic` and `ui`): no dialog, page tool or tool panel that only
/// a hidden command opens stays open, whatever opened it.
pub(crate) fn enforce(app: &mut PdfCraftApp) {
    // CI witness only (`--features evora-witness-enforce`, never in the web build): the guard is off.
    if cfg!(feature = "evora-witness-enforce") {
        return;
    }
    if app.dialog.as_ref().and_then(dialog_command).is_some_and(hidden) {
        app.dialog = None;
    }
    if tool_command(&app.quick_tool).is_some_and(hidden) {
        app.quick_tool = QuickTool::Select;
    }
    if let LeftPanel::Tool(id) = app.left
        && !tool_allowed(id)
    {
        app.left = LeftPanel::AllTools;
    }
}

impl PdfCraftApp {
    /// Evora (EVO-CMD, COL-43): the registry's commands as this build offers them, in registry order
    /// (the menus and the command palette list the same table), hidden commands left out:
    /// `(id, label, menu, enabled)`, with the label and the menu's name as displayed in the app's
    /// language, `menu` `None` for a command no menu shows (palette and tool panels only), and
    /// `enabled` as its menu entry is now. The host bridge picks among them what Evora may run.
    pub fn evora_commands(&self) -> Vec<(&'static str, String, Option<String>, bool)> {
        crate::i18n::set_current(crate::i18n::Lang::from_pref(&self.language));
        let active = self.active_ids().map(|(_, id)| id);
        pdfcraft_engine::commands::COMMANDS
            .iter()
            .filter(|spec| !hidden(spec.id))
            .map(|spec| {
                let label = pdfcraft_engine::commands::current_label(spec, &self.session, active);
                let label = crate::i18n::menu_label(spec.id, &label);
                let menu = spec.menu.map(|m| crate::i18n::t(m).to_string());
                (spec.id, label, menu, self.command_enabled(spec))
            })
            .collect()
    }
}

/// Evora (EVORA-COL-58, F4; EVORA-COL-63): the host bridge's answer to Evora's
/// `evora:request-save` for one tab (HOST-PROTOCOL.md §4.3 and §4.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostSaveAnswer {
    /// No such tab: `evora:ack {ok: false, code: "not_found"}`.
    NotFound,
    /// Nothing to save: `evora:ack {ok: true, code: "clean"}`.
    Clean,
    /// Save now ([`PdfCraftApp::save_for_host`]): `evora:save`, or `evora:ack {ok: false, code}` with
    /// the code it returns when the engine refuses what the member typed.
    Save,
}

/// `evora:ack` code of a save the engine refuses because the added text being typed has a
/// character no PDF font of this build can draw (it stays open, with its warning).
pub const REFUSED_TEXT: &str = "unsupported";
/// `evora:ack` code of a save the engine refuses because a form field refuses what was typed in
/// it (its format or validation script; the field tells why and stays open with the text).
pub const REFUSED_FIELD: &str = "invalid_content";

/// Evora (EVORA-COL-63, F4): a save Evora acknowledged whose bytes carried the member's typing
/// ([`PdfCraftApp::save_for_host`]): the document's edit generation then, and that typing.
pub(crate) struct HostTypedSave {
    pub(crate) id: DocId,
    pub(crate) generation: u64,
    pub(crate) edits: Vec<Edit>,
}

impl PdfCraftApp {
    /// Evora (EVORA-COL-58, F4): whether the member is typing in tab `index`: new text being added, a
    /// paragraph open for editing, a form field or the Fill & Sign type box open, or a text field of
    /// the interface holding the keyboard.
    pub fn typing_session(&self, index: usize) -> bool {
        let Some(v) = self.views.get(index) else { return false };
        v.content.draft.is_some()
            || v.line_editor.is_some()
            || v.forms.focus.is_some()
            || v.fill_text.is_some()
            || self.ctx.as_ref().is_some_and(|ctx| ctx.text_edit_focused())
    }

    /// Evora (EVORA-COL-63, F4): how the bridge answers Evora's `evora:request-save` for tab `index`,
    /// whatever its reason (`autosave`, `visibility`, `user`, `close`, `logout`): nothing to save
    /// (the typing included) is `Clean`, anything else saves. No save waits for the typing any more
    /// (EVORA-COL-58's `busy`, which held every automatic save while a text field kept the keyboard).
    pub fn host_save_answer(&self, index: usize) -> HostSaveAnswer {
        if index >= self.views.len() {
            return HostSaveAnswer::NotFound;
        }
        if !self.has_unsaved_work(index) {
            return HostSaveAnswer::Clean;
        }
        HostSaveAnswer::Save
    }

    /// Evora (EVORA-COL-63, F4): the edits committing the typing in tab `index` would apply
    /// (`commit_typing_in`), in its order, nothing committed: a queued edit, a form field's text, the
    /// Fill & Sign type box, new or retyped added text, an open paragraph. `Err` with the first
    /// character of added text that no PDF font of this build can draw: such text can't be saved.
    pub(crate) fn typing_edits(&self, index: usize) -> Result<Vec<Edit>, char> {
        let Some(v) = self.views.get(index) else { return Ok(Vec::new()) };
        let Some(doc) = self.session.get(v.id) else { return Ok(Vec::new()) };
        if let Some(c) = v.content.blocked() {
            return Err(c);
        }
        let mut edits: Vec<Edit> = v.pending_edit.iter().cloned().collect();
        edits.extend(v.forms.focus.as_ref().and_then(|f| crate::forms_ui::draft_edit(f, &doc.form)));
        if let Some(tb) = v.fill_text.as_ref().filter(|t| !t.text.trim().is_empty()) {
            let rotation = crate::comments::page_rotation(&doc.info, tb.page);
            edits.push(crate::fill_sign::typed(tb.page, tb.at, tb.text.trim(), tb.size, rotation, &self.comment_prefs.author));
        }
        edits.extend(v.content.draft_edit(doc.added()));
        edits.extend(v.line_editor.clone().filter(|ed| ed.has_changes()).and_then(crate::edit_text_ui::finished_edit));
        Ok(edits)
    }

    /// Evora (EVORA-COL-63, F4): Evora acknowledged a save of tab `index` as it is now: the document
    /// unchanged since, and the same typing.
    pub(crate) fn typed_save_holds(&self, index: usize) -> bool {
        let Some(saved) = self.host_typed_save.as_ref() else { return false };
        let Some(v) = self.views.get(index).filter(|v| v.id == saved.id) else { return false };
        self.session.get(v.id).is_some_and(|d| d.edit_generation() == saved.generation)
            && self.typing_edits(index).is_ok_and(|edits| edits == saved.edits)
    }

    /// Evora (EVORA-COL-63, F4): save tab `index` for Evora (`evora:request-save`, whatever its
    /// reason, and the member's own Save in the Atelier). What is being typed goes with the bytes
    /// ([`pdfcraft_engine::Session::save_bytes_with`]) and stays open: cursor, keyboard and tool
    /// unchanged, the next letters go on into the text. The document itself is left as it is until
    /// the member commits the typing; Evora's acknowledgement then holds while neither changes.
    /// Without typing, or without Evora, Save as before. `Ok` as [`Self::save_view`]; `Err` with the
    /// `evora:ack` code when the engine refuses the typing ([`REFUSED_TEXT`], [`REFUSED_FIELD`]).
    pub fn save_for_host(&mut self, index: usize) -> Result<bool, &'static str> {
        let Some(id) = self.views.get(index).map(|v| v.id) else { return Err("not_found") };
        let edits = match self.typing_edits(index) {
            Ok(edits) => edits,
            Err(_) => {
                // As before: the text, its warning and Discard are brought on screen (`commit_typing_in`).
                let _ = self.commit_typing_in(id);
                return Err(REFUSED_TEXT);
            }
        };
        // TÉMOIN SABORDÉ (EVORA-COL-66, S1) : la saisie est validée, puis enregistrée (le défaut F4).
        if !edits.is_empty() || edits.is_empty() || self.host_save_ack.is_none() {
            return self.commit_then_save(index, id);
        }
        let mut all = edits.clone();
        if self.flatten_fill_sign_on_save {
            all.push(Edit::FlattenFillSign);
        }
        match self.session.save_bytes_with(id, &all) {
            Ok(bytes) => Ok(self.send_typed(id, bytes, edits)),
            // What the document refuses (a field's format or script…) is committed as before, which
            // tells the member why and keeps the text.
            Err(_) => self.commit_then_save(index, id),
        }
    }

    /// The typing committed, then Save as before (`save_view`).
    fn commit_then_save(&mut self, index: usize, id: DocId) -> Result<bool, &'static str> {
        if !self.commit_typing_in(id) {
            return Err(REFUSED_FIELD);
        }
        Ok(self.save_view(index, SaveTarget::InPlace))
    }

    /// The bytes of a save with typing handed to Evora (`host_save_ack`): its outcome is applied by
    /// `apply_host_save_outcomes`; a host that doesn't acknowledge saves holds them at once.
    fn send_typed(&mut self, id: DocId, bytes: std::sync::Arc<Vec<u8>>, edits: Vec<Edit>) -> bool {
        let Some((name, generation)) = self.session.get(id).map(|d| (d.name.clone(), d.edit_generation())) else { return false };
        let Some(save) = self.host_save_ack.as_ref() else { return false };
        match save(id.0, &name, &bytes, false) {
            Ok(Some(ticket)) => {
                self.pending_host_saves.push(crate::PendingHostSave { ticket, id, bytes, generation, typing: Some(edits), after: Box::new(|_: &mut PdfCraftApp| {}) });
                false
            }
            Ok(None) => {
                self.host_typed_save = Some(HostTypedSave { id, generation, edits });
                true
            }
            Err(e) => {
                self.notify_fmt("Couldn't save {name}: {e}", &[("name", &name), ("e", &e)]);
                false
            }
        }
    }

    /// CI only (the web bridge's `window.__evoraRun`, `?evora-audit=1`): Add text typing without a
    /// keyboard, in the active tab: with `start`, a new text box on page 1 holding `text`, the Add text
    /// tool picked; otherwise `text` typed at the end of the open box. Returns whether it did. The box
    /// doesn't ask for the keyboard (EVORA-COL-64): the test's page has none to give, and a box that
    /// takes the keyboard and loses it at once is committed (`content_ui::editor`).
    pub fn evora_ci_type(&mut self, text: &str, start: bool) -> bool {
        let Some(i) = self.active else { return false };
        let Some(view) = self.views.get_mut(i) else { return false };
        if start {
            let style = pdfcraft_engine::AddedText { size: 14.0, ..Default::default() };
            let draft = crate::content_ui::TextDraft { page: 0, index: None, rect: [24.0, 60.0, 280.0, 80.0], text: text.to_string(), style, focus: false };
            view.content.draft = Some(draft);
            self.quick_tool = QuickTool::AddText;
            return true;
        }
        match view.content.draft.as_mut() {
            Some(d) => {
                d.text.push_str(text);
                true
            }
            None => false,
        }
    }
}

/// Evora (EVORA-COL-58, F5): whether a face of `family`, as `fonts` define it, has `c` in its character
/// map, for the CI checks of drawn glyphs (`tests/evora_finitions.rs`, the web build's render audit).
/// epaint's own `has_glyph` answers false for every character of the face that also holds its
/// replacement glyph, Inter's letters included.
pub fn family_has_char(fonts: &egui::FontDefinitions, family: &egui::FontFamily, c: char) -> bool {
    use skrifa::MetadataProvider as _;
    fonts.families.get(family).is_some_and(|stack| {
        stack
            .iter()
            .filter_map(|name| fonts.font_data.get(name))
            .any(|data| skrifa::FontRef::from_index(&data.font, data.index).is_ok_and(|face| face.charmap().map(c).is_some()))
    })
}

/// Evora (EVORA-COL-58, F5): the arrow of a submenu button. egui draws U+23F5, a glyph only its
/// emoji face has, and E10 takes that face out of the font stacks (`theme`): the arrow showed as
/// « ? ». U+203A is in Inter, the interface's face, and is no emoji.
pub(crate) const SUBMENU_ARROW: &str = "\u{203A}";

/// [`egui::Ui::menu_button`], a submenu's arrow drawn as [`SUBMENU_ARROW`] (a menu bar's or a
/// toolbar's menu button has no arrow and is egui's).
pub(crate) fn menu_button<'a, R>(
    ui: &mut egui::Ui,
    atoms: impl egui::IntoAtoms<'a>,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    if !egui::containers::menu::is_in_menu(ui) {
        return ui.menu_button(atoms, add_contents);
    }
    let (response, inner) = sub_menu_button(atoms).ui(ui, add_contents);
    egui::InnerResponse::new(inner.map(|i| i.inner), response)
}

/// [`egui::containers::menu::SubMenuButton::new`], with [`SUBMENU_ARROW`].
pub(crate) fn sub_menu_button<'a>(atoms: impl egui::IntoAtoms<'a>) -> egui::containers::menu::SubMenuButton<'a> {
    egui::containers::menu::SubMenuButton::from_button(egui::Button::new(atoms).right_text(SUBMENU_ARROW))
}
