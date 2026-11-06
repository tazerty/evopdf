//! Embeds the fonts of the optional evo-fonts build input (`EVO_FONTS_DIR`) as `EVO_FONTS`.
//! The recipe is evo-fonts' `docs/integration.md`; unset, `EVO_FONTS` is empty. It only reads
//! the local checkout (no network). On wasm32 it embeds only the UI faces (BIZ UDPGothic Regular
//! and any `Arab`, `Telu`, `Hebr`, `Thai` or `Deva` face), to keep the web build within hosting
//! limits (Cloudflare Pages: 25 MiB per file).

use std::fmt::Write as _;
use std::path::PathBuf;

#[path = "src/cjk_scripts.rs"]
mod cjk_scripts;

fn main() {
    println!("cargo::rerun-if-env-changed=EVO_FONTS_DIR");
    println!("cargo::rerun-if-env-changed=EVO_FONTS_REQUIRED");
    let mut src = String::from("pub static EVO_FONTS: &[EvoFont] = &[\n");
    if let Some(dir) = std::env::var_os("EVO_FONTS_DIR").map(PathBuf::from) {
        match evo_fonts(&dir) {
            Ok(entries) => src.push_str(&entries),
            Err(e) if std::env::var_os("EVO_FONTS_REQUIRED").is_some() => {
                println!("cargo::error=EVO_FONTS_DIR={}: {e}", dir.display());
            }
            Err(e) => println!("cargo::warning=building without evo-fonts: EVO_FONTS_DIR={}: {e}", dir.display()),
        }
    }
    src.push_str("];\n");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default()).join("evo_fonts.rs");
    if let Err(e) = std::fs::write(&out, src) {
        println!("cargo::error=writing {}: {e}", out.display());
    }
}

/// One `EvoFont { .. }` initialiser per manifest line.
fn evo_fonts(dir: &std::path::Path) -> Result<String, String> {
    let manifest = dir.join("fonts/manifest.txt");
    println!("cargo::rerun-if-changed={}", manifest.display());
    let text = std::fs::read_to_string(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let wasm = std::env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|a| a == "wasm32");
    let mut out = String::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let f: Vec<&str> = line.split(" | ").map(str::trim).collect();
        let [family, style, file, scripts, ..] = f.as_slice() else {
            return Err(format!("malformed manifest line: {line}"));
        };
        // Whole CJK faces aren't embedded for size; interface CJK comes from cut faces once the app
        // moves to evo-fonts-build (vendor/evo-libs#213).
        let cjk = scripts.split(',').any(cjk_scripts::is_cjk_script);
        if cjk
            && !cjk_scripts::WHOLE_CJK_FAMILIES.contains(family)
            && std::fs::metadata(dir.join(file)).is_ok_and(|m| m.len() > cjk_scripts::MAX_OTHER_CJK_BYTES)
        {
            continue;
        }
        // Arabic interface faces are small, so the web build keeps them too.
        let arabic = scripts.split(',').any(|s| s.trim() == "Arab");
        // Telugu faces too, for the Telugu interface.
        let telugu = scripts.split(',').any(|s| s.trim() == "Telu");
        // And the Hebrew ones (small), for Hebrew text and Edit text on the web.
        let hebrew = scripts.split(',').any(|s| s.trim() == "Hebr");
        // Devanagari too, for the Hindi interface (one small face).
        let devanagari = scripts.split(',').any(|s| s.trim() == "Deva");
        let web_face = (*family == "BIZ UDPGothic" && *style == "Regular")
            || arabic
            || telugu
            || hebrew
            || scripts.split(',').any(|s| s.trim() == "Thai")
            || devanagari;
        if wasm && !web_face {
            continue;
        }
        let path = dir.join(file).canonicalize().map_err(|e| format!("{file}: {e}"))?;
        println!("cargo::rerun-if-changed={}", path.display());
        let scripts: Vec<String> = scripts.split(',').map(|s| format!("{:?}", s.trim())).collect();
        let _ = writeln!(
            out,
            "    EvoFont {{ family: {family:?}, style: {style:?}, scripts: &[{}], bytes: include_bytes!({:?}) }},",
            scripts.join(", "),
            path.display().to_string(),
        );
    }
    Ok(out)
}
