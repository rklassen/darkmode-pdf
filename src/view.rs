use std::path::Path;

use anyhow::{Context, Result};
use image::GenericImageView;
use kurbo::{Point as KPoint, Rect as KRect};
use peniko::Color as PColor;
use printpdf::path::PaintMode;
use printpdf::{
    Color, Image, ImageTransform, LineDashPattern, LineJoinStyle, Mm, PdfLayerReference, Point,
    Polygon, Rect as PdfRect, Rgb,
};
use time::OffsetDateTime;

use crate::constants::{
    BLOCK_GAP_MM, BODY_FONT_PT, GOLDEN_RATIO, MARGIN_BOTTOM_MM, MARGIN_TOP_MM, PAGE_HEIGHT_MM,
    PAGE_WIDTH_MM, TABLE_GRID_MM,
};
use crate::model::{Construct, PageState, RenderCtx};
use crate::viewmodel::color::canonical_gray_hex;
use crate::viewmodel::text::{content_width_mm, pick_print_font, pt_to_mm};

const BEZIER_CIRCLE_FACTOR: f64 = 0.551_915_024_494;

pub(crate) fn fits_on_current_page(page: &PageState, needed_h_mm: f64) -> bool {
    page.y_mm - needed_h_mm >= MARGIN_BOTTOM_MM
}

pub(crate) fn ensure_space(
    ctx: &mut RenderCtx,
    page: &mut PageState,
    needed_h_mm: f64,
) -> Result<()> {
    if fits_on_current_page(page, needed_h_mm) {
        return Ok(());
    }
    start_new_page(ctx, page)
}

pub(crate) fn start_new_page(ctx: &mut RenderCtx, page: &mut PageState) -> Result<()> {
    let (page_idx, layer_idx) = ctx.doc.add_page(ctx.page_w, ctx.page_h, "darkmode-layer");
    let layer = ctx.doc.get_page(page_idx).get_layer(layer_idx);
    *page = PageState {
        layer,
        y_mm: PAGE_HEIGHT_MM - MARGIN_TOP_MM,
    };
    draw_page_background(ctx, &page.layer)?;
    Ok(())
}

pub(crate) fn draw_page_background(ctx: &RenderCtx, layer: &PdfLayerReference) -> Result<()> {
    draw_rect(
        layer,
        KRect::new(0.0, 0.0, PAGE_WIDTH_MM, PAGE_HEIGHT_MM),
        ctx.theme.page_bg,
    );

    if let Some(path) = &ctx.background_image {
        draw_cover_image(layer, path, 0.0, 0.0, PAGE_WIDTH_MM, PAGE_HEIGHT_MM)?;
    }
    Ok(())
}

pub(crate) fn spacer_mm(from: Option<Construct>, to: Construct) -> f64 {
    let _ = from;
    if matches!(to, Construct::Heading) {
        return 0.0;
    }
    BLOCK_GAP_MM
}

pub(crate) fn apply_spacer(
    ctx: &mut RenderCtx,
    page: &mut PageState,
    from: Option<Construct>,
    to: Construct,
) -> Result<()> {
    let Some(_) = from else {
        return Ok(());
    };
    let spacer = spacer_mm(from, to);
    if spacer <= 0.0 {
        return Ok(());
    }
    ensure_space(ctx, page, spacer)?;
    let spacer_top = page.y_mm;
    draw_spacer_square(
        &page.layer,
        ctx.content_box,
        spacer_top,
        spacer,
        canonical_gray_hex('0', 0.0).expect("valid hex gray"),
    );
    page.y_mm -= spacer;
    Ok(())
}

pub(crate) fn content_bounds_rect(ctx: &RenderCtx, top: f64, bottom: f64) -> KRect {
    KRect::new(ctx.content_box.x0, bottom, ctx.content_box.x1, top)
}

pub(crate) fn draw_construct_bounds(layer: &PdfLayerReference, rect: KRect) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    draw_rect_outline(layer, rect, PColor::from_rgba8(255, 0, 255, 255), 0.45);
}

pub(crate) fn draw_heading_bounds(layer: &PdfLayerReference, rect: KRect) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }

    layer.save_graphics_state();
    set_outline_color(layer, PColor::from_rgba8(0, 0, 0, 255));
    layer.set_outline_thickness(0.45);
    let mut dash_pattern = LineDashPattern::default();
    dash_pattern.dash_1 = Some(3);
    dash_pattern.gap_1 = Some(3);
    layer.set_line_dash_pattern(dash_pattern);
    layer.add_rect(
        PdfRect::new(
            Mm(rect.x0 as f32),
            Mm(rect.y0 as f32),
            Mm(rect.x1 as f32),
            Mm(rect.y1 as f32),
        )
        .with_mode(PaintMode::Stroke),
    );
    layer.restore_graphics_state();
}

pub(crate) fn code_block_radius_mm() -> f64 {
    pt_to_mm(BODY_FONT_PT * GOLDEN_RATIO.recip() * GOLDEN_RATIO.recip() * GOLDEN_RATIO.recip())
}

pub(crate) fn draw_spacer_square(
    layer: &PdfLayerReference,
    content_box: KRect,
    spacer_top: f64,
    spacer_height: f64,
    color: PColor,
) {
    if spacer_height <= 0.0 {
        return;
    }

    let side = spacer_height.min(content_box.width()).max(0.0);
    if side <= 0.0 {
        return;
    }

    let x = content_box.x0 + (content_box.width() - side) * 0.5;
    let y = spacer_top - (spacer_height + side) * 0.5;
    draw_rect(layer, KRect::new(x, y, x + side, y + side), color);
}

pub(crate) fn title_metadata_table_height_mm() -> f64 {
    let title_rank_pt = BODY_FONT_PT * GOLDEN_RATIO * GOLDEN_RATIO * GOLDEN_RATIO;
    let rank1_mm = pt_to_mm(GOLDEN_RATIO.recip() * title_rank_pt);
    let line_rank_mm = pt_to_mm(GOLDEN_RATIO);
    let outer_pad = rank1_mm / 9.0;
    let inner_pad = rank1_mm / 9.0;
    let row_gap = rank1_mm / 9.0;
    let extra_line_rank = line_rank_mm;
    let label_size = 7.5_f64;
    let value_size = 7.8_f64;
    let label_line_h = pt_to_mm(label_size * 1.35);
    let value_line_h = pt_to_mm(value_size * 1.35);
    let text_box_h = label_line_h.max(value_line_h) + extra_line_rank;
    let row_h = text_box_h + inner_pad * 2.0;
    outer_pad * 2.0 + row_h * 2.0 + row_gap
}

pub(crate) fn render_title_metadata_table_block(
    ctx: &mut RenderCtx,
    page: &mut PageState,
) -> Result<()> {
    let now = OffsetDateTime::now_utc();
    let zulu = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );

    let inset_w = content_width_mm(ctx);
    let title_rank_pt = BODY_FONT_PT * GOLDEN_RATIO * GOLDEN_RATIO * GOLDEN_RATIO;
    let rank1_mm = pt_to_mm(GOLDEN_RATIO.recip() * title_rank_pt);
    let line_rank_mm = pt_to_mm(GOLDEN_RATIO);
    let outer_pad = rank1_mm / 9.0;
    let inner_pad = rank1_mm / 9.0;
    let row_gap = rank1_mm / 9.0;
    let accent_gap = rank1_mm / 9.0;
    let accent_w = (rank1_mm * 0.72) / 9.0;
    let extra_line_rank = line_rank_mm;
    let label_size = 7.5_f64;
    let value_size = 7.8_f64;
    let label_line_h = pt_to_mm(label_size * 1.35);
    let value_line_h = pt_to_mm(value_size * 1.35);
    let text_box_h = label_line_h.max(value_line_h) + extra_line_rank;
    let row_h = text_box_h + inner_pad * 2.0;
    let inset_h = title_metadata_table_height_mm();
    ensure_space(ctx, page, inset_h)?;

    let inset_x = ctx.content_box.x0;
    let inset_top = page.y_mm;
    let inset = KRect::new(inset_x, inset_top - inset_h, inset_x + inset_w, inset_top);
    let label_w = inset_w * 0.22;
    let content_x0 = inset.x0 + outer_pad + accent_w + accent_gap;
    let content_x1 = inset.x1 - outer_pad;
    let divider_x = content_x0 + label_w;
    let value_x0 = divider_x + TABLE_GRID_MM;
    let row1_top = inset.y1 - outer_pad;
    let row1_bottom = row1_top - row_h;
    let row2_top = row1_bottom - row_gap;
    let row2_bottom = row2_top - row_h;
    let label_cell_bg = PColor::from_rgba8(46, 51, 58, 255);
    let value_cell_bg = PColor::from_rgba8(52, 57, 65, 255);

    draw_rect(
        &page.layer,
        KRect::new(content_x0, row1_bottom, divider_x, row1_top),
        label_cell_bg,
    );
    draw_rect(
        &page.layer,
        KRect::new(value_x0, row1_bottom, content_x1, row1_top),
        value_cell_bg,
    );
    draw_rect(
        &page.layer,
        KRect::new(content_x0, row2_bottom, divider_x, row2_top),
        label_cell_bg,
    );
    draw_rect(
        &page.layer,
        KRect::new(value_x0, row2_bottom, content_x1, row2_top),
        value_cell_bg,
    );
    draw_rect(
        &page.layer,
        KRect::new(
            divider_x,
            row2_bottom + inner_pad,
            divider_x + TABLE_GRID_MM,
            row1_top - inner_pad,
        ),
        PColor::from_rgba8(0, 0, 0, 255),
    );

    let rows = [("Zulu", zulu), ("Source", ctx.source_name.clone())];
    let style = crate::model::InlineStyle::default();
    let font = pick_print_font(&ctx.fonts.print, style, crate::model::TextRole::Body, false);
    set_fill_color(&page.layer, ctx.theme.muted);

    for (idx, (label, value)) in rows.iter().enumerate() {
        let row_top = if idx == 0 { row1_top } else { row2_top };
        let baseline = row_top - inner_pad - extra_line_rank - (text_box_h * 0.12);
        page.layer.use_text(
            *label,
            label_size as f32,
            Mm((content_x0 + inner_pad) as f32),
            Mm(baseline as f32),
            font,
        );
        page.layer.use_text(
            value.as_str(),
            value_size as f32,
            Mm((value_x0 + inner_pad) as f32),
            Mm(baseline as f32),
            font,
        );
    }

    draw_rect(
        &page.layer,
        KRect::new(
            inset.x0 + outer_pad,
            row2_bottom,
            inset.x0 + outer_pad + accent_w,
            row1_top,
        ),
        ctx.theme.text,
    );

    page.y_mm -= inset_h;
    draw_construct_bounds(
        &page.layer,
        content_bounds_rect(ctx, inset_top, inset_top - inset_h),
    );
    Ok(())
}

pub(crate) fn render_image_block(
    ctx: &mut RenderCtx,
    page: &mut PageState,
    image_path: &Path,
) -> Result<()> {
    let dyn_img = image::open(image_path)
        .with_context(|| format!("failed to open image: {}", image_path.display()))?;
    let (px_w, px_h) = dyn_img.dimensions();
    if px_w == 0 || px_h == 0 {
        return Ok(());
    }

    let max_w = content_width_mm(ctx);
    let max_h = (PAGE_HEIGHT_MM - MARGIN_TOP_MM - MARGIN_BOTTOM_MM) * 0.55;
    let ratio = px_w as f64 / px_h as f64;
    let mut draw_w = max_w;
    let mut draw_h = draw_w / ratio;
    if draw_h > max_h {
        draw_h = max_h;
        draw_w = draw_h * ratio;
    }

    ensure_space(ctx, page, draw_h)?;

    let img = Image::from_dynamic_image(&dyn_img);
    let x = ctx.content_box.x0 + (max_w - draw_w) * 0.5;
    let top = page.y_mm;
    let y_bottom = top - draw_h;
    let dpi = 300.0;
    let natural_w_mm = (px_w as f64 / dpi) * 25.4;
    let natural_h_mm = (px_h as f64 / dpi) * 25.4;
    let scale_x = draw_w / natural_w_mm;
    let scale_y = draw_h / natural_h_mm;

    img.add_to_layer(
        layer_clone(&page.layer),
        ImageTransform {
            translate_x: Some(Mm(x as f32)),
            translate_y: Some(Mm(y_bottom as f32)),
            rotate: None,
            scale_x: Some(scale_x as f32),
            scale_y: Some(scale_y as f32),
            dpi: Some(dpi as f32),
        },
    );

    page.y_mm -= draw_h;
    draw_construct_bounds(&page.layer, KRect::new(x, y_bottom, x + draw_w, top));
    Ok(())
}

fn draw_cover_image(
    layer: &PdfLayerReference,
    image_path: &Path,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<()> {
    let dyn_img = image::open(image_path)
        .with_context(|| format!("failed to open background image: {}", image_path.display()))?;
    let (px_w, px_h) = dyn_img.dimensions();
    if px_w == 0 || px_h == 0 {
        return Ok(());
    }

    let img = Image::from_dynamic_image(&dyn_img);
    let dpi = 300.0;
    let natural_w_mm = (px_w as f64 / dpi) * 25.4;
    let natural_h_mm = (px_h as f64 / dpi) * 25.4;
    let scale = f64::max(w / natural_w_mm, h / natural_h_mm);
    let draw_w = natural_w_mm * scale;
    let draw_h = natural_h_mm * scale;
    let draw_x = x + (w - draw_w) * 0.5;
    let draw_y = y + (h - draw_h) * 0.5;

    img.add_to_layer(
        layer_clone(layer),
        ImageTransform {
            translate_x: Some(Mm(draw_x as f32)),
            translate_y: Some(Mm(draw_y as f32)),
            rotate: None,
            scale_x: Some(scale as f32),
            scale_y: Some(scale as f32),
            dpi: Some(dpi as f32),
        },
    );

    Ok(())
}

pub(crate) fn draw_rect(layer: &PdfLayerReference, rect: KRect, color: PColor) {
    set_fill_color(layer, color);
    let rect = PdfRect::new(
        Mm(rect.x0 as f32),
        Mm(rect.y0 as f32),
        Mm(rect.x1 as f32),
        Mm(rect.y1 as f32),
    )
    .with_mode(PaintMode::Fill);
    layer.add_rect(rect);
}

pub(crate) fn draw_rect_outline(
    layer: &PdfLayerReference,
    rect: KRect,
    color: PColor,
    thickness_mm: f64,
) {
    let thickness = thickness_mm.max(0.1);
    draw_rect(
        layer,
        KRect::new(rect.x0, rect.y1 - thickness, rect.x1, rect.y1),
        color,
    );
    draw_rect(
        layer,
        KRect::new(rect.x0, rect.y0, rect.x1, rect.y0 + thickness),
        color,
    );
    draw_rect(
        layer,
        KRect::new(rect.x0, rect.y0, rect.x0 + thickness, rect.y1),
        color,
    );
    draw_rect(
        layer,
        KRect::new(rect.x1 - thickness, rect.y0, rect.x1, rect.y1),
        color,
    );
}

pub(crate) fn draw_rounded_rect(
    layer: &PdfLayerReference,
    rect: KRect,
    radius_mm: f64,
    color: PColor,
) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    set_fill_color(layer, color);
    layer.add_polygon(rounded_rect_polygon(rect, radius_mm, PaintMode::Fill));
}

pub(crate) fn draw_rounded_rect_outline(
    layer: &PdfLayerReference,
    rect: KRect,
    radius_mm: f64,
    color: PColor,
    thickness_mm: f64,
) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    set_outline_color(layer, color);
    layer.set_outline_thickness(thickness_mm as f32);
    layer.set_line_join_style(LineJoinStyle::Round);
    layer.add_polygon(rounded_rect_polygon(rect, radius_mm, PaintMode::Stroke));
}

fn rounded_rect_polygon(rect: KRect, radius_mm: f64, mode: PaintMode) -> Polygon {
    let radius = radius_mm
        .min(rect.width() * 0.5)
        .min(rect.height() * 0.5)
        .max(0.0);
    if radius <= 0.0 {
        return Polygon {
            rings: vec![vec![
                (Point::new(Mm(rect.x0 as f32), Mm(rect.y1 as f32)), false),
                (Point::new(Mm(rect.x1 as f32), Mm(rect.y1 as f32)), false),
                (Point::new(Mm(rect.x1 as f32), Mm(rect.y0 as f32)), false),
                (Point::new(Mm(rect.x0 as f32), Mm(rect.y0 as f32)), false),
            ]],
            mode,
            winding_order: printpdf::path::WindingOrder::NonZero,
        };
    }

    let c = radius * BEZIER_CIRCLE_FACTOR;
    let x0 = rect.x0;
    let x1 = rect.x1;
    let y0 = rect.y0;
    let y1 = rect.y1;
    let r = radius;

    let ring = vec![
        (mm_point(x0 + r, y1), false),
        (mm_point(x1 - r, y1), true),
        (mm_point(x1 - r + c, y1), true),
        (mm_point(x1, y1 - r + c), true),
        (mm_point(x1, y1 - r), false),
        (mm_point(x1, y0 + r), true),
        (mm_point(x1, y0 + r - c), true),
        (mm_point(x1 - r + c, y0), true),
        (mm_point(x1 - r, y0), false),
        (mm_point(x0 + r, y0), true),
        (mm_point(x0 + r - c, y0), true),
        (mm_point(x0, y0 + r - c), true),
        (mm_point(x0, y0 + r), false),
        (mm_point(x0, y1 - r), true),
        (mm_point(x0, y1 - r + c), true),
        (mm_point(x0 + r - c, y1), true),
        (mm_point(x0 + r, y1), false),
    ];

    Polygon {
        rings: vec![ring],
        mode,
        winding_order: printpdf::path::WindingOrder::NonZero,
    }
}

fn mm_point(x: f64, y: f64) -> Point {
    Point::new(Mm(x as f32), Mm(y as f32))
}

pub(crate) fn set_fill_color(layer: &PdfLayerReference, color: PColor) {
    let rgba = color.to_rgba8();
    layer.set_fill_color(Color::Rgb(Rgb::new(
        f32::from(rgba.r) / 255.0,
        f32::from(rgba.g) / 255.0,
        f32::from(rgba.b) / 255.0,
        None,
    )));
}

pub(crate) fn set_outline_color(layer: &PdfLayerReference, color: PColor) {
    let rgba = color.to_rgba8();
    layer.set_outline_color(Color::Rgb(Rgb::new(
        f32::from(rgba.r) / 255.0,
        f32::from(rgba.g) / 255.0,
        f32::from(rgba.b) / 255.0,
        None,
    )));
}

pub(crate) fn layer_clone(layer: &PdfLayerReference) -> PdfLayerReference {
    layer.clone()
}

#[allow(dead_code)]
pub(crate) fn _kpoint_to_mm(p: KPoint) -> (Mm, Mm) {
    (Mm(p.x as f32), Mm(p.y as f32))
}
