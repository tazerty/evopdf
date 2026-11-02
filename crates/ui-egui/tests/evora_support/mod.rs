//! Evora: what the Colophon Atelier's recipes share (`tests/evora_finitions.rs`,
//! `tests/evora_saisie.rs`): a verdict that collects every fault, one-page fixtures, the shell
//! opened on a document, and a member's click and typing.

#![allow(dead_code)]

use egui_kittest::Harness;
use pdfcraft_ui_egui::PdfCraftApp;

/// Collects faults, so that a red run names every one of them, and prints its count of checks,
/// which must not change from run to run.
pub struct Verdict {
    name: &'static str,
    checks: usize,
    faults: Vec<String>,
}

impl Verdict {
    pub fn new(name: &'static str) -> Self {
        Self { name, checks: 0, faults: Vec::new() }
    }

    pub fn holds(&mut self, ok: bool, fault: impl FnOnce() -> String) {
        self.checks += 1;
        if !ok {
            self.faults.push(fault());
        }
    }

    pub fn finish(self) {
        eprintln!("[{}] {} contrôle(s), {} faute(s)", self.name, self.checks, self.faults.len());
        for f in &self.faults {
            eprintln!("[{}] FAUTE {f}", self.name);
        }
        assert!(self.faults.is_empty(), "{} : {} faute(s) sur {} contrôle(s), détail ci-dessus", self.name, self.faults.len(), self.checks);
    }
}

/// A PDF of `objects` (object 1 the catalog), with its cross-reference table.
pub fn pdf(objects: &[String]) -> Vec<u8> {
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

/// The fixture's page width, in points (its MediaBox).
pub const PAGE_W: f32 = 300.0;

/// One page of 300 × 200 points with one line of text near its top (« Atelier », Helvetica 14,
/// baseline at 150 points), the rest of the page empty.
pub fn fixture() -> Vec<u8> {
    let body = "BT /F1 14 Tf 20 150 Td (Atelier) Tj ET";
    pdf(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 300 200] >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
        format!("<< /Length {} >>\nstream\n{body}\nendstream", body.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ])
}

/// The shell, in English, with `bytes` open as `name`.
pub fn harness_with(name: &'static str, bytes: Vec<u8>) -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 1000.0)).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.open_bytes(name, None, bytes.clone()).expect("the fixture opens");
        app
    });
    // Fonts install on frame 1 and apply on frame 2.
    h.run_steps(4);
    h
}

/// The shell on [`fixture`].
pub fn harness() -> Harness<'static, PdfCraftApp> {
    harness_with("finitions.pdf", fixture())
}

/// A slow click at `(x, y)` points of page 1, from its top left corner, as a member clicks (the
/// page [`PAGE_W`] points wide).
pub fn click_page(h: &mut Harness<'static, PdfCraftApp>, x: f32, y: f32) {
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
pub fn type_text(h: &mut Harness<'static, PdfCraftApp>, text: &str) {
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
