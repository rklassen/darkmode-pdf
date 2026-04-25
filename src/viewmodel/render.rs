use std::path::Path;

use anyhow::{Context, Result};
use peniko::Color as PColor;

use crate::constants::{BODY_FONT_PT, MARGIN_BOTTOM_MM, MARGIN_TOP_MM, PAGE_HEIGHT_MM};
use crate::model::{Block, InlineRun, PageState, RenderCtx, TextRole};
use crate::view::{
    content_bounds_rect, draw_construct_bounds, draw_heading_bounds, draw_spacer_square,
    ensure_space, render_image_block,
};
use crate::viewmodel::assets::resolve_path;
use crate::viewmodel::code::{estimate_code_block_keep_with_previous_mm, render_code_block};
use crate::viewmodel::color::{blend_srgb_over, canonical_gray_hex};
use crate::viewmodel::input::{heading_role, heading_size_pt};
use crate::viewmodel::table::{estimate_table_keep_with_previous_mm, render_table_block};
use crate::viewmodel::text::{
    content_width_mm, draw_text_lines, draw_text_lines_with_offset, pt_to_mm, wrap_runs,
};

pub(crate) fn render_block(
    ctx: &mut RenderCtx,
    page: &mut PageState,
    block: &Block,
    base_dir: &Path,
) -> Result<()> {
    match block {
        Block::Heading { level, runs } => render_heading(ctx, page, *level, runs),
        Block::Paragraph { runs } => render_paragraph(ctx, page, runs),
        Block::CodeFence { lang, code } => render_code_block(ctx, page, lang.as_deref(), code),
        Block::ListItem { depth, runs } => render_list_item(ctx, page, *depth, runs),
        Block::Image { path } => {
            let resolved = resolve_path(base_dir, path);
            render_image_block(ctx, page, &resolved)
        }
        Block::Table {
            alignments,
            headers,
            rows,
        } => render_table_block(ctx, page, alignments, headers, rows),
    }
}

pub(crate) fn heading_total_height_mm(ctx: &RenderCtx, level: u8, runs: &[InlineRun]) -> f64 {
    let size_pt = heading_size_pt(level);
    let role = heading_role(level);
    let lines = wrap_runs(
        ctx,
        &heading_display_runs(level, runs),
        size_pt,
        content_width_mm(ctx),
        false,
        role,
    );
    let heading_h = lines.len() as f64 * pt_to_mm(size_pt * 1.2);
    heading_h * (1.0 + 1.618)
}

pub(crate) fn estimate_block_keep_with_previous_mm(
    ctx: &RenderCtx,
    block: &Block,
    base_dir: &Path,
) -> Result<f64> {
    match block {
        Block::Heading { level, runs } => Ok(heading_total_height_mm(ctx, *level, runs)),
        Block::Paragraph { runs } => Ok(estimate_text_block_keep_with_previous_mm(
            ctx,
            runs,
            BODY_FONT_PT,
            1.6,
            content_width_mm(ctx),
        )),
        Block::CodeFence { lang, code } => Ok(estimate_code_block_keep_with_previous_mm(
            ctx,
            lang.as_deref(),
            code,
        )),
        Block::ListItem { depth, runs } => {
            let indent = *depth as f64 * 7.5;
            let avail_w = (content_width_mm(ctx) - indent).max(20.0);
            Ok(estimate_text_block_keep_with_previous_mm(
                ctx,
                runs,
                BODY_FONT_PT,
                1.55,
                avail_w,
            ))
        }
        Block::Image { path } => {
            estimate_image_keep_with_previous_mm(ctx, &resolve_path(base_dir, path))
        }
        Block::Table {
            alignments,
            headers,
            rows,
        } => Ok(estimate_table_keep_with_previous_mm(
            ctx, alignments, headers, rows,
        )),
    }
}

fn render_heading(
    ctx: &mut RenderCtx,
    page: &mut PageState,
    level: u8,
    runs: &[crate::model::InlineRun],
) -> Result<()> {
    let size_pt = heading_size_pt(level);
    let role = heading_role(level);
    let display_runs = heading_display_runs(level, runs);
    let color = if level == 1 {
        canonical_gray_hex('F', 0.0).expect("valid hex gray")
    } else {
        ctx.theme.heading_text
    };
    let line_height = pt_to_mm(size_pt * 1.2);
    let lines = wrap_runs(
        ctx,
        &display_runs,
        size_pt,
        content_width_mm(ctx),
        false,
        role,
    );
    let heading_h = lines.len() as f64 * line_height;
    let spacer_h = heading_h * 1.618;
    ensure_space(ctx, page, spacer_h + heading_h)?;
    let spacer_top = page.y_mm;
    draw_spacer_square(
        &page.layer,
        ctx.content_box,
        spacer_top,
        spacer_h,
        blend_srgb_over(
            ctx.theme.page_bg,
            canonical_gray_hex('0', 0.0).expect("valid hex gray"),
            0.38,
        ),
    );
    page.y_mm -= spacer_h;
    let top = page.y_mm;
    draw_text_lines(ctx, page, &lines, size_pt, line_height, color, role);
    draw_heading_bounds(&page.layer, content_bounds_rect(ctx, top, page.y_mm));
    Ok(())
}

fn heading_display_runs(level: u8, runs: &[InlineRun]) -> Vec<InlineRun> {
    let mut display_runs = Vec::with_capacity(runs.len() + 1);
    display_runs.push(InlineRun::plain(format!("{} ", "#".repeat(level as usize))));
    display_runs.extend_from_slice(runs);
    display_runs
}

fn estimate_text_block_keep_with_previous_mm(
    ctx: &RenderCtx,
    runs: &[InlineRun],
    size_pt: f64,
    line_height_multiple: f64,
    width_mm: f64,
) -> f64 {
    let line_height = pt_to_mm(size_pt * line_height_multiple);
    let lines = wrap_runs(ctx, runs, size_pt, width_mm, false, TextRole::Body);
    lines.len().clamp(1, 2) as f64 * line_height
}

fn estimate_image_keep_with_previous_mm(ctx: &RenderCtx, image_path: &Path) -> Result<f64> {
    let (px_w, px_h) = image::image_dimensions(image_path)
        .with_context(|| format!("failed to inspect image: {}", image_path.display()))?;
    if px_w == 0 || px_h == 0 {
        return Ok(0.0);
    }

    let max_w = content_width_mm(ctx);
    let max_h = (PAGE_HEIGHT_MM - MARGIN_TOP_MM - MARGIN_BOTTOM_MM) * 0.55;
    let ratio = px_w as f64 / px_h as f64;
    let mut draw_h = max_w / ratio;
    if draw_h > max_h {
        draw_h = max_h;
    }
    Ok(draw_h)
}

fn render_paragraph(
    ctx: &mut RenderCtx,
    page: &mut PageState,
    runs: &[crate::model::InlineRun],
) -> Result<()> {
    let size_pt = BODY_FONT_PT;
    let line_height = pt_to_mm(size_pt * 1.6);
    let lines = wrap_runs(
        ctx,
        runs,
        size_pt,
        content_width_mm(ctx),
        false,
        TextRole::Body,
    );
    ensure_space(ctx, page, lines.len() as f64 * line_height)?;
    let top = page.y_mm;
    draw_text_lines(
        ctx,
        page,
        &lines,
        size_pt,
        line_height,
        ctx.theme.text,
        TextRole::Body,
    );
    draw_construct_bounds(&page.layer, content_bounds_rect(ctx, top, page.y_mm));
    Ok(())
}

fn render_list_item(
    ctx: &mut RenderCtx,
    page: &mut PageState,
    depth: usize,
    runs: &[crate::model::InlineRun],
) -> Result<()> {
    let size_pt = BODY_FONT_PT;
    let line_height = pt_to_mm(size_pt * 1.55);
    let indent = depth as f64 * 7.5;
    let avail_w = (content_width_mm(ctx) - indent).max(20.0);
    let lines = wrap_runs(ctx, runs, size_pt, avail_w, false, TextRole::Body);
    ensure_space(ctx, page, lines.len() as f64 * line_height)?;
    let top = page.y_mm;

    let bullet_x = ctx.content_box.x0 + indent;
    let bullet_y = page.y_mm - line_height * 0.76;
    crate::view::draw_rect(
        &page.layer,
        kurbo::Rect::new(bullet_x, bullet_y, bullet_x + 1.4, bullet_y + 1.4),
        PColor::from_rgba8(126, 151, 136, 255),
    );

    draw_text_lines_with_offset(
        ctx,
        page,
        &lines,
        size_pt,
        line_height,
        ctx.theme.text,
        TextRole::Body,
        indent + 4.0,
    );
    draw_construct_bounds(&page.layer, content_bounds_rect(ctx, top, page.y_mm));
    Ok(())
}
