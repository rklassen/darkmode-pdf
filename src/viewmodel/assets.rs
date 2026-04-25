use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use printpdf::PdfDocumentReference;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use ttf_parser::Face;

use crate::model::{FontAssets, MeasureFonts, PrintFonts, SyntaxAssets};

pub(crate) fn load_syntax_assets() -> Result<SyntaxAssets> {
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();
    let theme = theme_set
        .themes
        .get("base16-ocean.dark")
        .or_else(|| theme_set.themes.get("Solarized (dark)"))
        .or_else(|| theme_set.themes.values().next())
        .cloned()
        .ok_or_else(|| anyhow!("failed to load syntect theme"))?;

    Ok(SyntaxAssets { syntax_set, theme })
}

pub(crate) fn resolve_path(base_dir: &Path, maybe_relative: &str) -> PathBuf {
    let p = Path::new(maybe_relative);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base_dir.join(p)
    }
}

fn find_font_path(root: &Path, candidates: &[&str]) -> Result<PathBuf> {
    for candidate in candidates {
        let path = root.join(candidate);
        if path.exists() {
            return Ok(path);
        }
    }
    bail!(
        "missing font; tried: {}",
        candidates
            .iter()
            .map(|name| root.join(name).display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

fn find_font_in_roots(candidates: &[&str]) -> Result<PathBuf> {
    let roots = [Path::new("assets"), Path::new("assets/fonts")];
    for root in roots {
        if let Ok(path) = find_font_path(root, candidates) {
            return Ok(path);
        }
    }
    bail!(
        "missing font; tried: {}",
        roots
            .iter()
            .flat_map(|root| {
                candidates
                    .iter()
                    .map(move |name| root.join(name).display().to_string())
            })
            .collect::<Vec<_>>()
            .join(", ")
    );
}

pub(crate) fn load_fonts(doc: &PdfDocumentReference) -> Result<FontAssets> {
    let body_regular_path = find_font_in_roots(&["FunnelSans-Light.ttf", "FunnelSans-Light.otf"])?;
    let body_italic_path =
        find_font_in_roots(&["FunnelSans-LightItalic.ttf", "FunnelSans-LightItalic.otf"])?;
    let body_bold_path = find_font_in_roots(&["FunnelSans-Bold.ttf", "FunnelSans-Bold.otf"])?;
    let heading_path = find_font_in_roots(&["Oswald-Light.ttf", "Oswald-Light.otf"])?;
    let heading_heavy_path = find_font_in_roots(&[
        "FunnelSans-ExtraBold.ttf",
        "FunnelSans-ExtraBold.otf",
        "FunnelSans-Bold.ttf",
        "FunnelSans-Bold.otf",
    ])?;
    let code_path = find_font_in_roots(&[
        "JetBrainsMono-Thin.ttf",
        "JetBrainsMono-Light.ttf",
        "JetBrainsMono-Light.otf",
    ])?;

    let body_regular_bytes = fs::read(&body_regular_path)
        .with_context(|| format!("missing font: {}", body_regular_path.display()))?;
    let body_italic_bytes = fs::read(&body_italic_path)
        .with_context(|| format!("missing font: {}", body_italic_path.display()))?;
    let body_bold_bytes = fs::read(&body_bold_path)
        .with_context(|| format!("missing font: {}", body_bold_path.display()))?;
    let heading_bytes = fs::read(&heading_path)
        .with_context(|| format!("missing font: {}", heading_path.display()))?;
    let heading_heavy_bytes = fs::read(&heading_heavy_path)
        .with_context(|| format!("missing font: {}", heading_heavy_path.display()))?;
    let code_bytes =
        fs::read(&code_path).with_context(|| format!("missing font: {}", code_path.display()))?;

    let body_regular_for_print = doc
        .add_external_font(std::io::Cursor::new(body_regular_bytes.clone()))
        .context("failed to embed FunnelSans-Light")?;
    let body_italic_for_print = doc
        .add_external_font(std::io::Cursor::new(body_italic_bytes.clone()))
        .context("failed to embed FunnelSans-LightItalic")?;
    let body_bold_for_print = doc
        .add_external_font(std::io::Cursor::new(body_bold_bytes.clone()))
        .context("failed to embed FunnelSans-Bold")?;
    let heading_for_print = doc
        .add_external_font(std::io::Cursor::new(heading_bytes.clone()))
        .context("failed to embed Oswald-Light")?;
    let heading_heavy_for_print = doc
        .add_external_font(std::io::Cursor::new(heading_heavy_bytes.clone()))
        .context("failed to embed FunnelSans heavy")?;
    let code_for_print = doc
        .add_external_font(std::io::Cursor::new(code_bytes.clone()))
        .context("failed to embed JetBrainsMono")?;

    Face::parse(&body_regular_bytes, 0).map_err(|_| anyhow!("invalid FunnelSans-Light"))?;
    Face::parse(&body_italic_bytes, 0).map_err(|_| anyhow!("invalid FunnelSans-LightItalic"))?;
    Face::parse(&body_bold_bytes, 0).map_err(|_| anyhow!("invalid FunnelSans-Bold"))?;
    Face::parse(&heading_bytes, 0).map_err(|_| anyhow!("invalid Oswald-Light"))?;
    Face::parse(&heading_heavy_bytes, 0).map_err(|_| anyhow!("invalid FunnelSans heavy"))?;
    Face::parse(&code_bytes, 0).map_err(|_| anyhow!("invalid JetBrainsMono"))?;

    Ok(FontAssets {
        print: PrintFonts {
            body_regular: body_regular_for_print,
            body_italic: body_italic_for_print,
            body_bold: body_bold_for_print,
            heading: heading_for_print,
            heading_heavy: heading_heavy_for_print,
            code: code_for_print,
        },
        measure: MeasureFonts {
            body_regular: body_regular_bytes,
            body_italic: body_italic_bytes,
            body_bold: body_bold_bytes,
            heading: heading_bytes,
            heading_heavy: heading_heavy_bytes,
            code: code_bytes,
        },
    })
}
