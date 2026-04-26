use anyhow::Result;
use kurbo::Rect as KRect;
use pulldown_cmark::Alignment;

use crate::constants::{PAGE_HEIGHT_MM, TABLE_CELL_PAD_X_MM, TABLE_CELL_PAD_Y_MM, TABLE_GRID_MM};
use crate::model::{InlineRun, PageState, RenderCtx, TableRowLayout, TextRole};
use crate::view::{draw_construct_bounds, draw_rect, fits_on_current_page, start_new_page};
use crate::viewmodel::measure::{
    measure_line_width_mm, measure_runs_longest_token_mm, measure_runs_max_line_width_mm,
};
use crate::viewmodel::text::{
    content_width_mm, effective_font_size_pt, inline_code_leading_gap_mm,
    inline_code_trailing_gap_mm, measure_run_width_mm, pick_print_font, pt_to_mm, wrap_runs,
};

pub(crate) fn render_table_block(
    ctx: &mut RenderCtx,
    page: &mut PageState,
    alignments: &[Alignment],
    headers: &[Vec<InlineRun>],
    rows: &[Vec<Vec<InlineRun>>],
) -> Result<()> {
    let mut col_count = alignments.len().max(headers.len());
    for row in rows {
        col_count = col_count.max(row.len());
    }
    if col_count == 0 {
        return Ok(());
    }

    let col_widths = compute_table_column_widths(ctx, col_count, headers, rows);
    let header_layout =
        (!headers.is_empty()).then(|| layout_table_row(ctx, headers, col_count, &col_widths, true));
    let body_layouts: Vec<TableRowLayout> = rows
        .iter()
        .map(|row| layout_table_row(ctx, row, col_count, &col_widths, false))
        .collect();
    let table_left = ctx.content_box.x0;
    let table_width: f64 = col_widths.iter().sum();

    let mut body_idx = 0usize;
    let mut draw_header_on_page = header_layout.is_some();
    let mut segment_top: Option<f64> = None;

    loop {
        if draw_header_on_page {
            if let Some(header) = &header_layout {
                let next_body_h = body_layouts
                    .get(body_idx)
                    .map(|row| row.height_mm)
                    .unwrap_or(0.0);
                let needed = header.height_mm + next_body_h;
                if !fits_on_current_page(page, needed) {
                    start_new_page(ctx, page)?;
                }
                segment_top.get_or_insert(page.y_mm);
                render_table_row(ctx, page, header, &col_widths, alignments, 0);
            }
            draw_header_on_page = false;
        }

        if body_idx >= body_layouts.len() {
            break;
        }

        let row = &body_layouts[body_idx];
        if !fits_on_current_page(page, row.height_mm)
            && page.y_mm < PAGE_HEIGHT_MM - crate::constants::MARGIN_TOP_MM
        {
            if let Some(top) = segment_top.take() {
                draw_construct_bounds(
                    &page.layer,
                    KRect::new(table_left, page.y_mm, table_left + table_width, top),
                );
            }
            start_new_page(ctx, page)?;
            draw_header_on_page = header_layout.is_some();
            continue;
        }

        segment_top.get_or_insert(page.y_mm);
        render_table_row(ctx, page, row, &col_widths, alignments, body_idx);
        body_idx += 1;
    }

    if let Some(top) = segment_top {
        draw_construct_bounds(
            &page.layer,
            KRect::new(table_left, page.y_mm, table_left + table_width, top),
        );
    }
    Ok(())
}

pub(crate) fn estimate_table_keep_with_previous_mm(
    ctx: &RenderCtx,
    alignments: &[Alignment],
    headers: &[Vec<InlineRun>],
    rows: &[Vec<Vec<InlineRun>>],
) -> f64 {
    let mut col_count = alignments.len().max(headers.len());
    for row in rows {
        col_count = col_count.max(row.len());
    }
    if col_count == 0 {
        return 0.0;
    }

    let col_widths = compute_table_column_widths(ctx, col_count, headers, rows);
    let header_h = (!headers.is_empty())
        .then(|| layout_table_row(ctx, headers, col_count, &col_widths, true).height_mm)
        .unwrap_or(0.0);
    let first_body_h = rows
        .first()
        .map(|row| layout_table_row(ctx, row, col_count, &col_widths, false).height_mm)
        .unwrap_or(0.0);
    if header_h > 0.0 {
        header_h + first_body_h
    } else {
        first_body_h.max(8.0)
    }
}

fn compute_table_column_widths(
    ctx: &RenderCtx,
    col_count: usize,
    headers: &[Vec<InlineRun>],
    rows: &[Vec<Vec<InlineRun>>],
) -> Vec<f64> {
    let available = content_width_mm(ctx);
    let mut min_widths = vec![14.0_f64; col_count];
    let mut pref_widths = vec![18.0_f64; col_count];

    let mut scan_row = |row: &[Vec<InlineRun>], force_bold: bool, size_pt: f64| {
        for col in 0..col_count {
            let cell = row.get(col).cloned().unwrap_or_default();
            let pref = measure_runs_max_line_width_mm(
                &ctx.fonts.measure,
                &cell,
                size_pt,
                TextRole::Body,
                force_bold,
            ) + 2.0 * TABLE_CELL_PAD_X_MM;
            let min_w = measure_runs_longest_token_mm(
                &ctx.fonts.measure,
                &cell,
                size_pt,
                TextRole::Body,
                force_bold,
            ) + 2.0 * TABLE_CELL_PAD_X_MM;
            min_widths[col] = min_widths[col].max(min_w.max(12.0));
            pref_widths[col] = pref_widths[col].max(pref.max(min_widths[col]));
        }
    };

    if !headers.is_empty() {
        scan_row(headers, true, 10.4);
    }
    for row in rows {
        scan_row(row, false, 10.2);
    }

    let pref_sum: f64 = pref_widths.iter().sum();
    let min_sum: f64 = min_widths.iter().sum();
    if pref_sum <= available {
        let extra = available - pref_sum;
        let per_col = extra / col_count as f64;
        return pref_widths.into_iter().map(|w| w + per_col).collect();
    }
    if min_sum >= available {
        let ratio = available / min_sum.max(1.0);
        return min_widths.into_iter().map(|w| w * ratio).collect();
    }

    let mut widths = min_widths.clone();
    let flex: Vec<f64> = pref_widths
        .iter()
        .zip(min_widths.iter())
        .map(|(pref, min_w)| (pref - min_w).max(0.0))
        .collect();
    let flex_sum: f64 = flex.iter().sum();
    let room = available - min_sum;

    if flex_sum <= 0.0 {
        let add = room / col_count as f64;
        widths.iter_mut().for_each(|w| *w += add);
        return widths;
    }
    for col in 0..col_count {
        widths[col] += room * (flex[col] / flex_sum);
    }
    widths
}

fn layout_table_row(
    ctx: &RenderCtx,
    row: &[Vec<InlineRun>],
    col_count: usize,
    col_widths: &[f64],
    is_header: bool,
) -> TableRowLayout {
    let font_size_pt = if is_header { 10.4 } else { 10.2 };
    let line_h_mm = pt_to_mm(font_size_pt * 1.45);
    let mut cells = Vec::with_capacity(col_count);
    let mut height_mm: f64 = 0.0;

    for col in 0..col_count {
        let runs = row.get(col).cloned().unwrap_or_default();
        let inner_w = (col_widths[col] - 2.0 * TABLE_CELL_PAD_X_MM).max(6.0);
        let lines = if runs.is_empty() {
            vec![Vec::new()]
        } else {
            wrap_runs(ctx, &runs, font_size_pt, inner_w, false, TextRole::Body)
        };
        height_mm = height_mm.max(lines.len() as f64 * line_h_mm + 2.0 * TABLE_CELL_PAD_Y_MM);
        cells.push(lines);
    }

    TableRowLayout {
        cells,
        height_mm: height_mm.max(8.0),
        font_size_pt,
        line_h_mm,
        is_header,
    }
}

fn render_table_row(
    ctx: &RenderCtx,
    page: &mut PageState,
    row: &TableRowLayout,
    col_widths: &[f64],
    alignments: &[Alignment],
    body_row_idx: usize,
) {
    let table_left = ctx.content_box.x0;
    let table_width: f64 = col_widths.iter().sum();
    let top = page.y_mm;
    let bottom = top - row.height_mm;
    let mut x = table_left;

    for (col, col_w) in col_widths.iter().enumerate() {
        let cell_rect = KRect::new(x, bottom, x + col_w, top);
        let cell_bg = if row.is_header {
            ctx.theme.table_header_bg
        } else if body_row_idx % 2 == 0 {
            ctx.theme.table_row_bg
        } else {
            ctx.theme.table_row_alt_bg
        };
        draw_rect(&page.layer, cell_rect, cell_bg);

        let align = alignments.get(col).copied().unwrap_or(Alignment::None);
        draw_table_cell_text(
            ctx,
            &page.layer,
            &row.cells[col],
            x,
            top,
            *col_w,
            row.font_size_pt,
            row.line_h_mm,
            align,
            row.is_header,
        );
        x += col_w;
    }

    draw_rect(
        &page.layer,
        KRect::new(
            table_left,
            top - TABLE_GRID_MM,
            table_left + table_width,
            top,
        ),
        ctx.theme.table_grid,
    );
    draw_rect(
        &page.layer,
        KRect::new(
            table_left,
            bottom,
            table_left + table_width,
            bottom + TABLE_GRID_MM,
        ),
        ctx.theme.table_grid,
    );

    let mut x_line = table_left;
    for col_w in col_widths {
        draw_rect(
            &page.layer,
            KRect::new(x_line, bottom, x_line + TABLE_GRID_MM, top),
            ctx.theme.table_grid,
        );
        x_line += col_w;
    }
    draw_rect(
        &page.layer,
        KRect::new(x_line - TABLE_GRID_MM, bottom, x_line, top),
        ctx.theme.table_grid,
    );
    page.y_mm -= row.height_mm;
}

fn draw_table_cell_text(
    ctx: &RenderCtx,
    layer: &printpdf::PdfLayerReference,
    lines: &[Vec<InlineRun>],
    cell_x: f64,
    cell_top: f64,
    cell_w: f64,
    size_pt: f64,
    line_h_mm: f64,
    alignment: Alignment,
    force_bold: bool,
) {
    crate::view::set_fill_color(layer, ctx.theme.text);
    let inner_w = (cell_w - 2.0 * TABLE_CELL_PAD_X_MM).max(0.0);
    let mut y = cell_top - TABLE_CELL_PAD_Y_MM;

    for line in lines {
        let line_w = measure_line_width_mm(
            &ctx.fonts.measure,
            line,
            size_pt,
            TextRole::Body,
            force_bold,
        );
        let x = match alignment {
            Alignment::Right => cell_x + cell_w - TABLE_CELL_PAD_X_MM - line_w,
            Alignment::Center => cell_x + TABLE_CELL_PAD_X_MM + ((inner_w - line_w) * 0.5).max(0.0),
            Alignment::Left | Alignment::None => cell_x + TABLE_CELL_PAD_X_MM,
        };

        let baseline = y - (line_h_mm * 0.85);
        let mut run_x = x;
        for (idx, run) in line.iter().enumerate() {
            if run.text.is_empty() {
                continue;
            }
            run_x += inline_code_leading_gap_mm(&ctx.fonts.measure, line, idx);
            let font = pick_print_font(&ctx.fonts.print, run.style, TextRole::Body, force_bold);
            let run_size_pt = effective_font_size_pt(size_pt, run.style);
            layer.use_text(
                &run.text,
                run_size_pt as f32,
                printpdf::Mm(run_x as f32),
                printpdf::Mm(baseline as f32),
                font,
            );
            run_x += measure_run_width_mm(
                &ctx.fonts.measure,
                &run.text,
                run.style,
                size_pt,
                TextRole::Body,
                force_bold,
            );
            run_x += inline_code_trailing_gap_mm(&ctx.fonts.measure, line, idx);
        }
        y -= line_h_mm;
    }
}
