use std::path::PathBuf;

use kurbo::Rect as KRect;
use peniko::Color as PColor;
use printpdf::{IndirectFontRef, PdfDocumentReference, PdfLayerReference};
use pulldown_cmark::Alignment;
use syntect::highlighting::Theme as SyntectTheme;
use syntect::parsing::SyntaxSet;

use crate::viewmodel::color::canonical_gray_hex;

#[derive(Clone)]
pub(crate) struct Theme {
    pub(crate) page_bg: PColor,
    pub(crate) text: PColor,
    pub(crate) heading_text: PColor,
    pub(crate) muted: PColor,
    pub(crate) code_bg: PColor,
    pub(crate) inline_code_bg: PColor,
    pub(crate) inline_code_outline: PColor,
    pub(crate) code_text: PColor,
    pub(crate) table_grid: PColor,
    pub(crate) table_header_bg: PColor,
    pub(crate) table_row_bg: PColor,
    pub(crate) table_row_alt_bg: PColor,
}

impl Theme {
    pub(crate) fn dark() -> Self {
        Self {
            page_bg: canonical_gray_hex('2', 0.0).expect("valid hex gray"),
            text: canonical_gray_hex('A', 0.0).expect("valid hex gray"),
            heading_text: canonical_gray_hex('D', 0.0).expect("valid hex gray"),
            muted: PColor::from_rgba8(148, 163, 184, 255),
            code_bg: PColor::from_rgba8(23, 23, 23, 255),
            inline_code_bg: canonical_gray_hex('3', 0.0).expect("valid hex gray"),
            inline_code_outline: canonical_gray_hex('5', 0.0).expect("valid hex gray"),
            code_text: PColor::from_rgba8(224, 242, 194, 255),
            table_grid: PColor::from_rgba8(68, 75, 85, 255),
            table_header_bg: PColor::from_rgba8(25, 27, 31, 255),
            table_row_bg: PColor::from_rgba8(20, 22, 25, 255),
            table_row_alt_bg: PColor::from_rgba8(18, 19, 22, 255),
        }
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct InlineStyle {
    pub(crate) italic: bool,
    pub(crate) bold: bool,
    pub(crate) code: bool,
}

#[derive(Clone)]
pub(crate) struct InlineRun {
    pub(crate) text: String,
    pub(crate) style: InlineStyle,
    pub(crate) link_url: Option<String>,
}

impl InlineRun {
    pub(crate) fn plain(text: String) -> Self {
        Self {
            text,
            style: InlineStyle::default(),
            link_url: None,
        }
    }
}

#[derive(Clone)]
pub(crate) enum Block {
    Heading {
        level: u8,
        runs: Vec<InlineRun>,
    },
    Paragraph {
        runs: Vec<InlineRun>,
    },
    CodeFence {
        lang: Option<String>,
        code: String,
    },
    ListItem {
        depth: usize,
        runs: Vec<InlineRun>,
    },
    Image {
        path: String,
    },
    Table {
        alignments: Vec<Alignment>,
        headers: Vec<Vec<InlineRun>>,
        rows: Vec<Vec<Vec<InlineRun>>>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Construct {
    Heading,
    Paragraph,
    CodeFence,
    ListItem,
    Image,
    Table,
    TitleMetadata,
}

impl Construct {
    pub(crate) fn from_block(block: &Block) -> Self {
        match block {
            Block::Heading { .. } => Self::Heading,
            Block::Paragraph { .. } => Self::Paragraph,
            Block::CodeFence { .. } => Self::CodeFence,
            Block::ListItem { .. } => Self::ListItem,
            Block::Image { .. } => Self::Image,
            Block::Table { .. } => Self::Table,
        }
    }
}

#[derive(Default)]
pub(crate) struct Frontmatter {
    pub(crate) background_image: Option<String>,
}

pub(crate) struct SourceDoc {
    pub(crate) frontmatter: Frontmatter,
    pub(crate) blocks: Vec<Block>,
}

#[derive(Clone)]
pub(crate) struct PrintFonts {
    pub(crate) body_regular: IndirectFontRef,
    pub(crate) body_italic: IndirectFontRef,
    pub(crate) body_bold: IndirectFontRef,
    pub(crate) heading: IndirectFontRef,
    pub(crate) heading_heavy: IndirectFontRef,
    pub(crate) code: IndirectFontRef,
}

#[derive(Clone)]
pub(crate) struct MeasureFonts {
    pub(crate) body_regular: Vec<u8>,
    pub(crate) body_italic: Vec<u8>,
    pub(crate) body_bold: Vec<u8>,
    pub(crate) heading: Vec<u8>,
    pub(crate) heading_heavy: Vec<u8>,
    pub(crate) code: Vec<u8>,
}

#[derive(Clone, Copy)]
pub(crate) enum TextRole {
    Body,
    Heading,
    HeadingHeavy,
}

pub(crate) struct FontAssets {
    pub(crate) print: PrintFonts,
    pub(crate) measure: MeasureFonts,
}

pub(crate) struct SyntaxAssets {
    pub(crate) syntax_set: SyntaxSet,
    pub(crate) theme: SyntectTheme,
}

pub(crate) struct RenderCtx {
    pub(crate) doc: PdfDocumentReference,
    pub(crate) fonts: FontAssets,
    pub(crate) syntax: SyntaxAssets,
    pub(crate) theme: Theme,
    pub(crate) page_w: printpdf::Mm,
    pub(crate) page_h: printpdf::Mm,
    pub(crate) content_box: KRect,
    pub(crate) background_image: Option<PathBuf>,
    pub(crate) source_name: String,
}

pub(crate) struct PageState {
    pub(crate) layer: PdfLayerReference,
    pub(crate) y_mm: f64,
}

#[derive(Clone)]
pub(crate) struct ColoredRun {
    pub(crate) text: String,
    pub(crate) color: PColor,
}

#[derive(Clone)]
pub(crate) struct TableRowLayout {
    pub(crate) cells: Vec<Vec<Vec<InlineRun>>>,
    pub(crate) height_mm: f64,
    pub(crate) font_size_pt: f64,
    pub(crate) line_h_mm: f64,
    pub(crate) is_header: bool,
}
