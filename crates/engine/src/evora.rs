//! Evora (EVORA-COL-63, F4): the bytes of a document as the member sees it while typing.
//!
//! Evora saves the Colophon Atelier's document on its own clock (autosave), when the tab is hidden,
//! before leaving and when the member asks. Committing what the member is typing first closed the
//! text box: the letters typed next went to the tools' keys (H picks the Hand). Here the typing is
//! applied to a copy of the document's graph, as [`Session::apply`] would apply it, and the copy is
//! written as [`Session::save_bytes`] writes the document; the document itself is left exactly as it
//! is: no undo step, no refresh, no new generation, nothing counted saved.

use super::*;

impl Session {
    /// The bytes [`Session::save_bytes`] would write once `edits` (what the member is typing, as
    /// committing it would apply it) were applied, document `id` left as it is. No `edits`: the
    /// bytes of `save_bytes`. A field's script or format that refuses its value, or an edit the
    /// document's permissions or signatures forbid, fails as `Session::apply` would.
    pub fn save_bytes_with(&self, id: DocId, edits: &[Edit]) -> Result<Arc<Vec<u8>>, EditError> {
        if edits.is_empty() {
            return self.save_bytes(id);
        }
        let doc = self.get(id).ok_or(EditError::NoDocument)?;
        let editor = doc.editor.as_ref().ok_or_else(|| EditError::ReadOnly(doc.read_only_reason.clone().unwrap_or_default()))?;
        let mut next = editor.cos.clone();
        for edit in edits {
            self.preview_edit(doc, editor, &mut next, edit)?;
        }
        // As `Session::apply`: a signed document is only ever saved incrementally.
        let rewrites = |c: &evopdf_cos::Document| c.full_save_required() || c.encryption_changed();
        if doc.is_signed() && ((rewrites(&next) && !rewrites(&editor.cos)) || (next.is_modified() && next.revisions().is_empty())) {
            return Err(EditError::SignedRewrite(edits.first().map(|e| e.label().to_string()).unwrap_or_default()));
        }
        self.write_save(editor, &next)
    }

    /// `edit` applied to `next`, a copy of document `doc`'s graph, as `Session::apply` applies it:
    /// the document's permissions, an OCR word's cover, field scripts, and an XFA form's data.
    fn preview_edit(&self, doc: &Document, editor: &Editor, next: &mut evopdf_cos::Document, edit: &Edit) -> Result<(), EditError> {
        if let Some(p) = next.permissions() {
            check_permission(edit, &p)?;
        }
        // Nothing to bake: as `Session::apply`, which leaves the document alone.
        if matches!(edit, Edit::FlattenFillSign) && !evopdf_annot::has_visible_fill_sign(next) {
            return Ok(());
        }
        let edit = covered(doc, editor, edit.clone());
        let mut cx = EditCtx::new(self.now(), doc.generation ^ (doc.id.0 << 48));
        cx.pages = doc.info.pages.len();
        cx.today = self.today();
        if !self.js_off {
            cx.xfa = doc.xfa_template.clone();
            if uses_scripts(&edit) {
                cx.js = Some(js::JsRunner::new(next, &doc.name));
            }
        }
        guard(|| run_edit(&mut *next, &edit, &mut cx))
            .unwrap_or_else(|m| Err(EditError::Invalid(format!("{} failed unexpectedly ({m}); the document was not changed", edit.label()))))?;
        // XFA forms keep their values in the datasets packet too, for Adobe's viewers.
        if doc.info.xfa.is_some() && matches!(scope_of(&edit), Scope::Form | Scope::Full | Scope::Content) {
            let (tpl, datasets) = (cx.xfa.as_deref(), &mut cx.xfa_datasets);
            guard(|| xfa_sync_datasets(&mut *next, tpl, datasets))
                .unwrap_or_else(|m| Err(format!("writing the XFA data failed unexpectedly ({m})")))
                .map_err(EditError::Write)?;
        }
        Ok(())
    }
}

/// Rewriting an OCR word (invisible text over a scan) also hides the scanned word, as
/// `Session::apply` does: cover it in the paper's colour and draw the new text in the ink's.
fn covered(doc: &Document, editor: &Editor, edit: Edit) -> Edit {
    match edit {
        Edit::EditTextBlock { page, block, text, mut style } => {
            if style.cover.is_none()
                && let Some(info) = doc.info.pages.get(page)
                && let Some(b) = editor.text_blocks(page).ok().and_then(|b| b.get(block).filter(|b| b.invisible).cloned())
                && let Some((paper, ink)) = guard(|| scan_colours::sample(&editor.cos, info, page, &b)).ok().flatten()
            {
                style.cover = Some(paper);
                style.color = style.color.or(Some(ink));
            }
            Edit::EditTextBlock { page, block, text, style }
        }
        other => other,
    }
}
