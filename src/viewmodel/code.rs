use anyhow::Result;
use peniko::Color as PColor;
use syntect::easy::HighlightLines;
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

use crate::constants::{BODY_FONT_PT, CODE_BLOCK_PADDING_MM, GOLDEN_RATIO, MARGIN_BOTTOM_MM};
use crate::model::{ColoredRun, InlineStyle, PageState, RenderCtx, TextRole};
use crate::view::{
    code_block_radius_mm, draw_rounded_rect, draw_rounded_rect_outline, set_fill_color,
    start_new_page,
};
use crate::viewmodel::color::syntect_color_to_peniko;
use crate::viewmodel::text::{
    content_width_mm, effective_font_size_pt, measure_run_width_mm, pick_print_font, pt_to_mm,
};

pub(crate) fn render_code_block(
    ctx: &mut RenderCtx,
    page: &mut PageState,
    lang: Option<&str>,
    code: &str,
) -> Result<()> {
    let size_pt = BODY_FONT_PT * GOLDEN_RATIO.recip();
    let line_height = pt_to_mm(size_pt * 1.55);
    let width_mm = content_width_mm(ctx);
    let lines = highlight_and_wrap_code(
        ctx,
        code,
        lang,
        size_pt,
        width_mm - CODE_BLOCK_PADDING_MM * 2.0,
    );
    let mut line_idx = 0usize;

    while line_idx < lines.len() {
        let first_segment = line_idx == 0;
        let top_pad = if first_segment {
            CODE_BLOCK_PADDING_MM
        } else {
            0.0
        };
        let available_mm = page.y_mm - MARGIN_BOTTOM_MM;
        if available_mm <= top_pad + line_height {
            start_new_page(ctx, page)?;
            continue;
        }

        let mut lines_fit = ((available_mm - top_pad) / line_height).floor().max(1.0) as usize;
        lines_fit = lines_fit.min(lines.len() - line_idx);

        let mut last_segment = line_idx + lines_fit == lines.len();
        let mut bottom_pad = if last_segment {
            CODE_BLOCK_PADDING_MM
        } else {
            0.0
        };
        let mut needed = top_pad + lines_fit as f64 * line_height + bottom_pad;
        while needed > available_mm && lines_fit > 1 {
            lines_fit -= 1;
            last_segment = line_idx + lines_fit == lines.len();
            bottom_pad = if last_segment {
                CODE_BLOCK_PADDING_MM
            } else {
                0.0
            };
            needed = top_pad + lines_fit as f64 * line_height + bottom_pad;
        }

        let segment_top = page.y_mm;
        let segment_bottom = segment_top - (top_pad + lines_fit as f64 * line_height + bottom_pad);
        let segment_rect = kurbo::Rect::new(
            ctx.content_box.x0,
            segment_bottom,
            ctx.content_box.x1,
            segment_top,
        );
        draw_rounded_rect(
            &page.layer,
            segment_rect,
            code_block_radius_mm(),
            ctx.theme.code_bg,
        );
        draw_rounded_rect_outline(
            &page.layer,
            segment_rect,
            code_block_radius_mm(),
            peniko::Color::from_rgba8(255, 0, 255, 255),
            0.45,
        );

        page.y_mm -= top_pad;
        draw_colored_code_lines(
            ctx,
            page,
            &lines[line_idx..line_idx + lines_fit],
            size_pt,
            line_height,
        );
        page.y_mm -= bottom_pad;
        line_idx += lines_fit;

        if !last_segment {
            start_new_page(ctx, page)?;
        }
    }
    Ok(())
}

pub(crate) fn estimate_code_block_keep_with_previous_mm(
    ctx: &RenderCtx,
    lang: Option<&str>,
    code: &str,
) -> f64 {
    let size_pt = BODY_FONT_PT * GOLDEN_RATIO.recip();
    let line_height = pt_to_mm(size_pt * 1.55);
    let width_mm = content_width_mm(ctx);
    let lines = highlight_and_wrap_code(
        ctx,
        code,
        lang,
        size_pt,
        width_mm - CODE_BLOCK_PADDING_MM * 2.0,
    );
    CODE_BLOCK_PADDING_MM + lines.len().clamp(1, 2) as f64 * line_height
}

fn highlight_and_wrap_code(
    ctx: &RenderCtx,
    code: &str,
    lang: Option<&str>,
    size_pt: f64,
    width_mm: f64,
) -> Vec<Vec<ColoredRun>> {
    let syntax = code_syntax_for_lang(&ctx.syntax.syntax_set, lang);
    let mut highlighter = HighlightLines::new(syntax, &ctx.syntax.theme);
    let mut out = Vec::new();

    for source_line in LinesWithEndings::from(code) {
        let mut raw_segments = Vec::new();
        match highlighter.highlight_line(source_line, &ctx.syntax.syntax_set) {
            Ok(regions) => {
                for (style, text) in regions {
                    let text = text.trim_end_matches(['\n', '\r']);
                    if text.is_empty() {
                        continue;
                    }
                    raw_segments.push(ColoredRun {
                        text: text.to_string(),
                        color: syntect_color_to_peniko(style.foreground),
                    });
                }
            }
            Err(_) => {
                raw_segments.push(ColoredRun {
                    text: source_line.trim_end_matches(['\n', '\r']).to_string(),
                    color: ctx.theme.code_text,
                });
            }
        }

        let wrapped = wrap_colored_runs(ctx, &raw_segments, size_pt, width_mm);
        if wrapped.is_empty() {
            out.push(Vec::new());
        } else {
            out.extend(wrapped);
        }
    }

    if code.is_empty() {
        out.push(Vec::new());
    }
    out
}

fn draw_colored_code_lines(
    ctx: &RenderCtx,
    page: &mut PageState,
    lines: &[Vec<ColoredRun>],
    size_pt: f64,
    line_h_mm: f64,
) {
    let code_style = InlineStyle {
        code: true,
        italic: false,
        bold: false,
    };
    let font = pick_print_font(&ctx.fonts.print, code_style, TextRole::Body, false);

    for line in lines {
        let mut x = ctx.content_box.x0 + CODE_BLOCK_PADDING_MM;
        let baseline = page.y_mm - (line_h_mm * 0.85);

        for run in line {
            if run.text.is_empty() {
                continue;
            }
            set_fill_color(&page.layer, run.color);
            page.layer.use_text(
                &run.text,
                effective_font_size_pt(size_pt, code_style) as f32,
                printpdf::Mm(x as f32),
                printpdf::Mm(baseline as f32),
                font,
            );
            x += measure_run_width_mm(
                &ctx.fonts.measure,
                &run.text,
                code_style,
                size_pt,
                TextRole::Body,
                false,
            );
        }

        page.y_mm -= line_h_mm;
    }
}

fn wrap_colored_runs(
    ctx: &RenderCtx,
    runs: &[ColoredRun],
    size_pt: f64,
    width_mm: f64,
) -> Vec<Vec<ColoredRun>> {
    let code_style = InlineStyle {
        code: true,
        italic: false,
        bold: false,
    };
    let mut lines: Vec<Vec<ColoredRun>> = vec![Vec::new()];
    let mut line_w = 0.0_f64;

    for run in runs {
        let mut buf = String::new();
        for ch in run.text.chars() {
            let ch_s = ch.to_string();
            let ch_w = measure_run_width_mm(
                &ctx.fonts.measure,
                &ch_s,
                code_style,
                size_pt,
                TextRole::Body,
                false,
            );

            if line_w > 0.0 && line_w + ch_w > width_mm {
                push_colored_run(lines.last_mut().expect("line exists"), &mut buf, run.color);
                lines.push(Vec::new());
                line_w = 0.0;
            }

            buf.push(ch);
            line_w += ch_w;
        }

        push_colored_run(lines.last_mut().expect("line exists"), &mut buf, run.color);
    }
    lines
}

fn push_colored_run(line: &mut Vec<ColoredRun>, buf: &mut String, color: PColor) {
    if buf.is_empty() {
        return;
    }
    if let Some(last) = line.last_mut() {
        if last.color == color {
            last.text.push_str(buf);
            buf.clear();
            return;
        }
    }
    line.push(ColoredRun {
        text: std::mem::take(buf),
        color,
    });
}

fn code_syntax_for_lang<'a>(syntax_set: &'a SyntaxSet, lang: Option<&str>) -> &'a SyntaxReference {
    let plain = syntax_set.find_syntax_plain_text();
    let Some(lang) = lang else {
        return plain;
    };

    let raw = lang.trim().to_ascii_lowercase();
    if raw.is_empty() {
        return plain;
    }
    let token = raw
        .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .next()
        .unwrap_or(raw.as_str());

    let mapped = match token {
        "rs" | "rust" => "rust",
        "py" | "python" => "python",
        "html" | "htm" => "html",
        "xml" => "xml",
        "md" | "markdown" => "markdown",
        "ts" | "typescript" => "typescript",
        "js" | "javascript" => "javascript",
        "json" => "json",
        other => other,
    };

    syntax_set
        .find_syntax_by_token(mapped)
        .or_else(|| syntax_set.find_syntax_by_extension(mapped))
        .unwrap_or(plain)
}
