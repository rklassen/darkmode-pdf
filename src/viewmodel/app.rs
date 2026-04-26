use std::env;
use std::fs;
use std::io::BufWriter;
use std::path::Path;

use anyhow::{Context, Result, bail};
use kurbo::Rect as KRect;
use printpdf::{Mm, PdfDocument};

use crate::constants::{
    MARGIN_BOTTOM_MM, MARGIN_LEFT_MM, MARGIN_RIGHT_MM, MARGIN_TOP_MM, PAGE_HEIGHT_MM, PAGE_WIDTH_MM,
};
use crate::model::{Block, Construct, PageState, RenderCtx, Theme};
use crate::view::{
    apply_spacer, draw_page_background, fits_on_current_page, render_title_metadata_table_block,
    spacer_mm, start_new_page, title_metadata_table_height_mm,
};
use crate::viewmodel::assets::{load_fonts, load_syntax_assets, resolve_path};
use crate::viewmodel::input::parse_markdown_with_frontmatter;
use crate::viewmodel::render::{
    estimate_block_keep_with_previous_mm, heading_total_height_mm, render_block,
};

pub(crate) fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        bail!("Usage: darkmode-pdf <input.md> <output.pdf>");
    }

    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);
    let input = fs::read_to_string(input_path)
        .with_context(|| format!("failed to read markdown: {}", input_path.display()))?;
    let src = parse_markdown_with_frontmatter(&input);

    let base_dir = input_path.parent().unwrap_or_else(|| Path::new("."));
    let source_name = input_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "source.md".to_string());
    let theme = Theme::dark();
    let syntax = load_syntax_assets()?;
    let page_w = Mm(PAGE_WIDTH_MM as f32);
    let page_h = Mm(PAGE_HEIGHT_MM as f32);
    let (doc, first_page, first_layer) =
        PdfDocument::new("darkmode markdown", page_w, page_h, "darkmode-layer-1");
    let fonts = load_fonts(&doc)?;

    let mut ctx = RenderCtx {
        doc,
        fonts,
        syntax,
        theme,
        page_w,
        page_h,
        content_box: KRect::new(
            MARGIN_LEFT_MM,
            MARGIN_BOTTOM_MM,
            PAGE_WIDTH_MM - MARGIN_RIGHT_MM,
            PAGE_HEIGHT_MM - MARGIN_TOP_MM,
        ),
        background_image: src
            .frontmatter
            .background_image
            .as_ref()
            .map(|raw| resolve_path(base_dir, raw)),
        source_name,
    };

    let mut page = PageState {
        layer: ctx.doc.get_page(first_page).get_layer(first_layer),
        y_mm: PAGE_HEIGHT_MM - MARGIN_TOP_MM,
    };

    draw_page_background(&ctx, &page.layer)?;

    let has_h1 = src
        .blocks
        .iter()
        .any(|b| matches!(b, Block::Heading { level: 1, .. }));
    let mut subtitle_rendered = false;
    let mut previous_construct: Option<Construct> = None;

    for (idx, block) in src.blocks.iter().enumerate() {
        let construct = Construct::from_block(block);
        if let Block::Heading { level, runs } = block {
            let keep_with_next = if !subtitle_rendered && *level == 1 {
                Some((Construct::TitleMetadata, title_metadata_table_height_mm()))
            } else if let Some(next_block) = src.blocks.get(idx + 1) {
                Some((
                    Construct::from_block(next_block),
                    estimate_block_keep_with_previous_mm(&ctx, next_block, base_dir)?,
                ))
            } else {
                None
            };

            if let Some((next_construct, next_height_mm)) = keep_with_next {
                let needed = heading_total_height_mm(&ctx, *level, runs)
                    + spacer_mm(Some(Construct::Heading), next_construct)
                    + next_height_mm;
                if needed <= ctx.content_box.height() && !fits_on_current_page(&page, needed) {
                    start_new_page(&mut ctx, &mut page)?;
                }
            }
        }
        apply_spacer(&mut ctx, &mut page, previous_construct, construct)?;
        render_block(&mut ctx, &mut page, block, base_dir)?;
        previous_construct = Some(construct);
        if !subtitle_rendered && matches!(block, Block::Heading { level: 1, .. }) {
            apply_spacer(
                &mut ctx,
                &mut page,
                previous_construct,
                Construct::TitleMetadata,
            )?;
            render_title_metadata_table_block(&mut ctx, &mut page)?;
            subtitle_rendered = true;
            previous_construct = Some(Construct::TitleMetadata);
        }
    }

    if !has_h1 && !subtitle_rendered {
        apply_spacer(
            &mut ctx,
            &mut page,
            previous_construct,
            Construct::TitleMetadata,
        )?;
        render_title_metadata_table_block(&mut ctx, &mut page)?;
    }

    let mut output_file = fs::File::create(output_path)
        .with_context(|| format!("failed to create output file: {}", output_path.display()))?;
    let mut writer = BufWriter::new(&mut output_file);
    ctx.doc
        .save(&mut writer)
        .with_context(|| format!("failed to write PDF: {}", output_path.display()))?;

    Ok(())
}
