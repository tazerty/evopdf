//! Evora (EVORA-COL-58, EVORA-COL-63): fixes of the Colophon Atelier's daily use, judged in the real
//! shell (egui_kittest, like `tests/evora_masks.rs`). Each test collects every fault before failing and
//! prints its count of checks, which must not change from run to run. (F4, typing through every save,
//! is judged in `tests/evora_saisie.rs` since EVORA-COL-63.)
//!
//! - « glyphes » (F5): the main menu open, no text it paints, its submenus' arrows included, has a
//!   character missing from the interface's fonts (egui's arrow U+23F5 had none since E10 took the
//!   emoji faces out, and showed as « ? »). Twin: the menu's submenus are drawn. Run with
//!   `PDFCRAFT_SYSTEM_FONTS=0`, so that no installed face stands in for a missing glyph.
//! - « garde-menus » (EVORA-COL-63, Evora's RC2-11): no source of the shell calls egui's
//!   `menu_button` or `SubMenuButton::new` itself: every submenu goes through `evora::menu_button`,
//!   whose arrow the interface's fonts draw. Twin: the wrapper's own calls are found.

mod evora_support;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use evora_support::{Verdict, harness};
use pdfcraft_ui_egui::{PdfCraftApp, family_has_char};

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

/// Every line of `dir`'s Rust sources, recursively, with its file, outside comments.
fn source_lines(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut paths: Vec<std::path::PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            source_lines(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let name = path.display().to_string();
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            out.extend(text.lines().filter(|l| !l.trim_start().starts_with("//")).map(|l| (name.clone(), l.to_string())));
        }
    }
}

#[test]
fn every_submenu_goes_through_the_evora_wrapper() {
    let mut v = Verdict::new("garde-menus");
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut lines = Vec::new();
    source_lines(&src, &mut lines);
    v.holds(lines.len() > 1000, || format!("prémisse : sources du shell introuvables ({} ligne(s) lues sous {})", lines.len(), src.display()));
    let wrapper = std::path::Path::new("src").join("evora.rs").display().to_string();
    let outside = |file: &str| !file.ends_with(&wrapper);
    let raw: Vec<String> = lines
        .iter()
        .filter(|(file, line)| outside(file) && (line.contains(".menu_button(") || line.contains("SubMenuButton::new")))
        .map(|(file, line)| format!("{file} : {}", line.trim()))
        .collect();
    let wrapped = lines.iter().filter(|(file, line)| outside(file) && line.contains("evora::menu_button(")).count();
    v.holds(wrapped >= 10, || format!("témoin : appels de evora::menu_button trouvés {wrapped} (au moins 10 attendus)"));
    v.holds(raw.is_empty(), || format!("sous-menu(s) d'egui appelé(s) hors de evora.rs (flèche U+23F5 sans glyphe) : {raw:?}"));
    v.finish();
}
