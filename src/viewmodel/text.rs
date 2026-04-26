use printpdf::{
    Actions, BorderArray, ColorArray, HighlightingMode, LinkAnnotation, Mm, Rect as PdfRect,
};
use ttf_parser::Face;

use crate::model::{
    InlineRun, InlineStyle, MeasureFonts, PageState, PrintFonts, RenderCtx, TextRole,
};
use crate::view::{
    code_block_radius_mm, draw_rounded_rect, draw_rounded_rect_outline, set_fill_color,
};

pub(crate) fn content_width_mm(ctx: &RenderCtx) -> f64 {
    ctx.content_box.width()
}

pub(crate) fn draw_text_lines(
    ctx: &RenderCtx,
    page: &mut PageState,
    lines: &[Vec<InlineRun>],
    size_pt: f64,
    line_h_mm: f64,
    color: peniko::Color,
    role: TextRole,
) {
    draw_text_lines_with_offset(ctx, page, lines, size_pt, line_h_mm, color, role, 0.0);
}

pub(crate) fn draw_text_lines_with_offset(
    ctx: &RenderCtx,
    page: &mut PageState,
    lines: &[Vec<InlineRun>],
    size_pt: f64,
    line_h_mm: f64,
    color: peniko::Color,
    role: TextRole,
    x_offset_mm: f64,
) {
    set_fill_color(&page.layer, color);

    for line in lines {
        let mut x = ctx.content_box.x0 + x_offset_mm;
        let baseline = page.y_mm - (line_h_mm * 0.85);

        for (idx, run) in line.iter().enumerate() {
            if run.text.is_empty() {
                continue;
            }

            let font = pick_print_font(&ctx.fonts.print, run.style, role, false);
            let run_size_pt = effective_font_size_pt(size_pt, run.style);
            let code_padding = inline_code_padding_mm(&ctx.fonts.measure, run.style);
            x += inline_code_leading_gap_mm(&ctx.fonts.measure, line, idx);
            let text_x = x + code_padding;
            if run.style.code {
                let chip_lift = inline_code_chip_lift_mm(line_h_mm);
                let chip_rect = kurbo::Rect::new(
                    x,
                    baseline - line_h_mm * 0.30 + chip_lift,
                    x + measure_inline_run_advance_mm(
                        &ctx.fonts.measure,
                        &run.text,
                        run.style,
                        size_pt,
                        role,
                        false,
                    ),
                    baseline + line_h_mm * 0.52 + chip_lift,
                );
                draw_rounded_rect(
                    &page.layer,
                    chip_rect,
                    code_block_radius_mm(),
                    ctx.theme.inline_code_bg,
                );
                draw_rounded_rect_outline(
                    &page.layer,
                    chip_rect,
                    code_block_radius_mm(),
                    ctx.theme.inline_code_outline,
                    pt_to_mm(0.38),
                );
                set_fill_color(&page.layer, color);
            }
            page.layer.use_text(
                &run.text,
                run_size_pt as f32,
                Mm(text_x as f32),
                Mm(baseline as f32),
                font,
            );

            let advance = measure_inline_run_advance_mm(
                &ctx.fonts.measure,
                &run.text,
                run.style,
                size_pt,
                role,
                false,
            );
            if let Some(url) = supported_link_uri(run.link_url.as_deref()) {
                let rect = PdfRect::new(
                    Mm(x as f32),
                    Mm((baseline - line_h_mm * 0.25) as f32),
                    Mm((x + advance) as f32),
                    Mm((baseline + line_h_mm * 0.75) as f32),
                );
                page.layer.add_link_annotation(LinkAnnotation::new(
                    rect,
                    Some(BorderArray::Solid([0.0, 0.0, 0.0])),
                    Some(ColorArray::Transparent),
                    Actions::uri(url.to_string()),
                    Some(HighlightingMode::Invert),
                ));
            }
            x += advance + inline_code_trailing_gap_mm(&ctx.fonts.measure, line, idx);
        }

        page.y_mm -= line_h_mm;
    }
}

pub(crate) fn inline_code_chip_lift_mm(line_h_mm: f64) -> f64 {
    let baseline_to_bottom_edge = line_h_mm * 0.30;
    baseline_to_bottom_edge * crate::constants::GOLDEN_RATIO.recip().powi(3)
}

pub(crate) fn effective_font_size_pt(size_pt: f64, style: InlineStyle) -> f64 {
    if style.code {
        size_pt * crate::constants::MONO_OPTICAL_ADJUSTMENT
    } else {
        size_pt
    }
}

pub(crate) fn inline_code_padding_mm(fonts: &MeasureFonts, style: InlineStyle) -> f64 {
    if style.code {
        let body_space = measure_run_width_mm(
            fonts,
            " ",
            InlineStyle::default(),
            crate::constants::BODY_FONT_PT,
            TextRole::Body,
            false,
        );
        body_space * (1.0 + crate::constants::GOLDEN_RATIO.recip())
    } else {
        0.0
    }
}

pub(crate) fn inline_code_outer_gap_mm(fonts: &MeasureFonts, style: InlineStyle) -> f64 {
    if style.code {
        let body_space = measure_run_width_mm(
            fonts,
            " ",
            InlineStyle::default(),
            crate::constants::BODY_FONT_PT,
            TextRole::Body,
            false,
        );
        body_space * crate::constants::GOLDEN_RATIO.recip()
    } else {
        0.0
    }
}

pub(crate) fn inline_code_leading_gap_mm(
    fonts: &MeasureFonts,
    line: &[InlineRun],
    idx: usize,
) -> f64 {
    if idx > 0 {
        inline_code_outer_gap_mm(fonts, line[idx].style)
    } else {
        0.0
    }
}

pub(crate) fn inline_code_trailing_gap_mm(
    fonts: &MeasureFonts,
    line: &[InlineRun],
    idx: usize,
) -> f64 {
    if idx + 1 < line.len() {
        inline_code_outer_gap_mm(fonts, line[idx].style)
    } else {
        0.0
    }
}

pub(crate) fn measure_inline_run_advance_mm(
    fonts: &MeasureFonts,
    text: &str,
    style: InlineStyle,
    size_pt: f64,
    role: TextRole,
    force_bold: bool,
) -> f64 {
    measure_run_width_mm(fonts, text, style, size_pt, role, force_bold)
        + inline_code_padding_mm(fonts, style) * 2.0
}

pub(crate) fn measure_line_run_advance_mm(
    fonts: &MeasureFonts,
    line: &[InlineRun],
    idx: usize,
    size_pt: f64,
    role: TextRole,
    force_bold: bool,
) -> f64 {
    let run = &line[idx];
    inline_code_leading_gap_mm(fonts, line, idx)
        + measure_inline_run_advance_mm(fonts, &run.text, run.style, size_pt, role, force_bold)
        + inline_code_trailing_gap_mm(fonts, line, idx)
}

fn supported_link_uri(value: Option<&str>) -> Option<&str> {
    let url = value?.trim();
    if url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:") {
        Some(url)
    } else {
        None
    }
}

pub(crate) fn wrap_runs(
    ctx: &RenderCtx,
    runs: &[InlineRun],
    size_pt: f64,
    width_mm: f64,
    preserve_newlines: bool,
    role: TextRole,
) -> Vec<Vec<InlineRun>> {
    let mut lines: Vec<Vec<InlineRun>> = vec![Vec::new()];
    let mut line_w = 0.0;

    for run in runs {
        let chunks = if run.style.code {
            vec![run.text.clone()]
        } else if preserve_newlines {
            split_keep_newlines(&run.text)
        } else {
            split_words_keep_space(&run.text)
        };

        for chunk in chunks {
            if chunk == "\n" {
                lines.push(Vec::new());
                line_w = 0.0;
                continue;
            }

            let mut measured = measure_inline_run_advance_mm(
                &ctx.fonts.measure,
                &chunk,
                run.style,
                size_pt,
                role,
                false,
            );
            if !lines.last().expect("line exists").is_empty() {
                if lines
                    .last()
                    .expect("line exists")
                    .last()
                    .is_some_and(|prev| prev.style.code)
                {
                    measured += inline_code_outer_gap_mm(
                        &ctx.fonts.measure,
                        lines
                            .last()
                            .expect("line exists")
                            .last()
                            .expect("run exists")
                            .style,
                    );
                }
                measured += inline_code_outer_gap_mm(&ctx.fonts.measure, run.style);
            }
            if line_w > 0.0 && line_w + measured > width_mm {
                lines.push(Vec::new());
                line_w = 0.0;
                if chunk.trim().is_empty() {
                    continue;
                }
            }

            if measured > width_mm {
                let parts = hard_wrap_chunk(
                    &ctx.fonts.measure,
                    &chunk,
                    run.style,
                    size_pt,
                    width_mm,
                    role,
                );
                for (idx, part) in parts.into_iter().enumerate() {
                    if idx > 0 {
                        lines.push(Vec::new());
                        line_w = 0.0;
                    }
                    let w = measure_run_width_mm(
                        &ctx.fonts.measure,
                        &part,
                        run.style,
                        size_pt,
                        role,
                        false,
                    );
                    lines.last_mut().expect("line exists").push(InlineRun {
                        text: part,
                        style: run.style,
                        link_url: run.link_url.clone(),
                    });
                    line_w += w + inline_code_padding_mm(&ctx.fonts.measure, run.style) * 2.0;
                }
                continue;
            }

            lines.last_mut().expect("line exists").push(InlineRun {
                text: chunk,
                style: run.style,
                link_url: run.link_url.clone(),
            });
            line_w += measured;
        }
    }

    while lines.len() > 1 && lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines
}

fn split_words_keep_space(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut in_space = false;

    for ch in text.chars() {
        if ch == '\n' {
            if !buf.is_empty() {
                out.push(std::mem::take(&mut buf));
            }
            out.push("\n".to_string());
            in_space = false;
            continue;
        }

        if ch.is_whitespace() {
            if !in_space && !buf.is_empty() {
                out.push(std::mem::take(&mut buf));
            }
            buf.push(ch);
            in_space = true;
        } else {
            if in_space && !buf.is_empty() {
                out.push(std::mem::take(&mut buf));
            }
            buf.push(ch);
            in_space = false;
        }
    }

    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

fn split_keep_newlines(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (idx, part) in text.split('\n').enumerate() {
        if idx > 0 {
            out.push("\n".to_string());
        }
        if !part.is_empty() {
            out.push(part.to_string());
        }
    }
    if text.ends_with('\n') {
        out.push("\n".to_string());
    }
    out
}

fn hard_wrap_chunk(
    fonts: &MeasureFonts,
    chunk: &str,
    style: InlineStyle,
    size_pt: f64,
    width_mm: f64,
    role: TextRole,
) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();

    for ch in chunk.chars() {
        buf.push(ch);
        let w = measure_run_width_mm(fonts, &buf, style, size_pt, role, false);
        if w > width_mm && buf.chars().count() > 1 {
            let last = buf.pop().expect("char exists");
            out.push(std::mem::take(&mut buf));
            buf.push(last);
        }
    }

    if !buf.is_empty() {
        out.push(buf);
    }
    if out.is_empty() {
        out.push(chunk.to_string());
    }
    out
}

pub(crate) fn pick_print_font<'a>(
    fonts: &'a PrintFonts,
    style: InlineStyle,
    role: TextRole,
    force_bold: bool,
) -> &'a printpdf::IndirectFontRef {
    if style.code {
        return &fonts.code;
    }
    match role {
        TextRole::Heading => return &fonts.heading,
        TextRole::HeadingHeavy => return &fonts.heading_heavy,
        TextRole::Body => {}
    }
    if force_bold || style.bold {
        return &fonts.body_bold;
    }
    if style.italic {
        return &fonts.body_italic;
    }
    &fonts.body_regular
}

fn pick_measure_font<'a>(
    fonts: &'a MeasureFonts,
    style: InlineStyle,
    role: TextRole,
    force_bold: bool,
) -> &'a [u8] {
    if style.code {
        return &fonts.code;
    }
    match role {
        TextRole::Heading => return &fonts.heading,
        TextRole::HeadingHeavy => return &fonts.heading_heavy,
        TextRole::Body => {}
    }
    if force_bold || style.bold {
        return &fonts.body_bold;
    }
    if style.italic {
        return &fonts.body_italic;
    }
    &fonts.body_regular
}

pub(crate) fn measure_run_width_mm(
    fonts: &MeasureFonts,
    text: &str,
    style: InlineStyle,
    size_pt: f64,
    role: TextRole,
    force_bold: bool,
) -> f64 {
    if text.is_empty() {
        return 0.0;
    }

    let font_bytes = pick_measure_font(fonts, style, role, force_bold);
    let face = match Face::parse(font_bytes, 0) {
        Ok(face) => face,
        Err(_) => return 0.0,
    };

    let units_per_em = face.units_per_em() as f64;
    if units_per_em <= 0.0 {
        return 0.0;
    }

    let mut total_units = 0.0_f64;
    for ch in text.chars() {
        let Some(glyph_id) = face.glyph_index(ch) else {
            total_units += units_per_em * 0.5;
            continue;
        };
        let advance = face
            .glyph_hor_advance(glyph_id)
            .unwrap_or((units_per_em * 0.5) as u16);
        total_units += f64::from(advance);
    }

    let width_pt = (total_units / units_per_em) * effective_font_size_pt(size_pt, style);
    pt_to_mm(width_pt)
}

pub(crate) fn pt_to_mm(pt: f64) -> f64 {
    pt * 25.4 / 72.0
}
