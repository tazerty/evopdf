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
//!   tool groups disappear. Tools ▸ Document JavaScripts… and Add a digital signature field (an
//!   empty field, no signature) stay, as decided before; they are left to Evora's arbitration.
//!
//! Two fixes of the bridge's daily use live here too (EVORA-COL-58): an automatic save asked by
//! Evora never interrupts typing ([`PdfCraftApp::host_save_answer`], F4), and a submenu's arrow is
//! drawn in a glyph the interface's fonts have ([`menu_button`], F5).
//!
//! Hidden commands and catalogue items stay out of the menus, the command palette, the keyboard
//! shortcuts list, the tool panels and Home, and do nothing if run another way; a tool group whose
//! items are all hidden disappears. Every frame, [`enforce`] closes a dialog, drops a page tool and
//! leaves a tool panel that only a hidden command opens, whatever opened it (a click on an empty
//! signature field, the Signatures panel, automation). Links found in documents are not affected:
//! they keep their confirmation dialog.

use pdfcraft_engine::catalog::{self, ToolGroup};

use crate::{Dialog, LeftPanel, PdfCraftApp, QuickTool};

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
    // SABORDAGE QA (EVORA-COL-61) S1 : les masques F6 de EVORA-COL-58 retirés (à retirer par le commit suivant).
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

/// Evora (EVORA-COL-58, F4): the host bridge's answer to Evora's `evora:request-save` for one tab
/// (HOST-PROTOCOL.md §4.3 and §4.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostSaveAnswer {
    /// No such tab: `evora:ack {ok: false, code: "not_found"}`.
    NotFound,
    /// Nothing to save: `evora:ack {ok: true, code: "clean"}`.
    Clean,
    /// An automatic save while the member types: `evora:ack {ok: false, code: "busy"}`. Nothing is
    /// committed, closed or moved off the keyboard; Evora keeps its state and asks again on its next
    /// cycle (§4.5), and the typing goes with that save or the member's own.
    Busy,
    /// Save now, typing committed first (#166), as before: `evora:save`.
    Save,
}

impl PdfCraftApp {
    /// Evora (EVORA-COL-58, F4): whether the member is typing in tab `index`: new text being added, a
    /// paragraph open for editing, a form field or the Fill & Sign type box open, or a text field of
    /// the interface holding the keyboard. A save commits each of them, which closes the box: the
    /// letters typed next would go to the tools' keys (H picks the Hand).
    pub fn typing_session(&self, index: usize) -> bool {
        let Some(v) = self.views.get(index) else { return false };
        v.content.draft.is_some()
            || v.line_editor.is_some()
            || v.forms.focus.is_some()
            || v.fill_text.is_some()
            || self.ctx.as_ref().is_some_and(|ctx| ctx.text_edit_focused())
    }

    /// Evora (EVORA-COL-58, F4): how the bridge answers Evora's `evora:request-save` for tab `index`.
    /// An automatic save (`autosave`, Evora's timer) waits while the member types: answering at once
    /// with `busy` keeps Evora's single save request free for a save the member asks for, and no
    /// unchanged version reaches Evora Drive. Every other reason (`user`, `close`, `logout`,
    /// `visibility`) saves as before, typing committed first.
    pub fn host_save_answer(&self, index: usize, autosave: bool) -> HostSaveAnswer {
        if index >= self.views.len() {
            return HostSaveAnswer::NotFound;
        }
        if !self.has_unsaved_work(index) {
            return HostSaveAnswer::Clean;
        }
        // SABORDAGE QA (EVORA-COL-61) S2 : garde F4 retirée (à retirer par le commit suivant).
        let _ = autosave;
        HostSaveAnswer::Save
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
// SABORDAGE QA (EVORA-COL-61) S3 : flèche d'egui U+23F5 rendue (à retirer par le commit suivant).
pub(crate) const SUBMENU_ARROW: &str = "\u{23F5}";

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
