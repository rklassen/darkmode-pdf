use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::constants::{BODY_FONT_PT, GOLDEN_RATIO};
use crate::model::{Block, Frontmatter, InlineRun, InlineStyle, SourceDoc, TextRole};

pub(crate) fn parse_markdown_with_frontmatter(markdown: &str) -> SourceDoc {
    let (frontmatter, body) = split_frontmatter(markdown);
    let mut blocks = Vec::new();
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(body, options);
    let mut stack: Vec<InlineStyle> = vec![InlineStyle::default()];
    let mut current_runs: Vec<InlineRun> = Vec::new();
    let mut current_heading_level: Option<u8> = None;
    let mut in_item = false;
    let mut list_depth: usize = 0;
    let mut in_code_fence = false;
    let mut code_fence_buf = String::new();
    let mut current_code_lang: Option<String> = None;
    let mut in_image = false;
    let mut image_dest = String::new();
    let mut link_stack: Vec<Option<String>> = Vec::new();
    let mut in_table = false;
    let mut in_table_head = false;
    let mut table_alignments = Vec::new();
    let mut table_headers = Vec::new();
    let mut table_rows = Vec::new();
    let mut current_table_row = Vec::new();
    let mut current_table_cell = Vec::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    if !in_table {
                        current_runs.clear();
                    }
                }
                Tag::Heading { level, .. } => {
                    current_runs.clear();
                    current_heading_level = Some(heading_level(level));
                }
                Tag::Emphasis => push_style(&mut stack, |s| s.italic = true),
                Tag::Strong => push_style(&mut stack, |s| s.bold = true),
                Tag::CodeBlock(kind) => {
                    in_code_fence = true;
                    code_fence_buf.clear();
                    current_code_lang = None;
                    if let CodeBlockKind::Fenced(lang) = kind {
                        let lang = lang.trim();
                        if !lang.is_empty() {
                            current_code_lang = Some(lang.to_ascii_lowercase());
                        }
                    }
                }
                Tag::Item => {
                    in_item = true;
                    current_runs.clear();
                }
                Tag::List(_) => {
                    list_depth += 1;
                }
                Tag::Image { dest_url, .. } => {
                    in_image = true;
                    image_dest = dest_url.to_string();
                }
                Tag::Link { dest_url, .. } => {
                    let dest = dest_url.trim();
                    let value = if dest.is_empty() {
                        None
                    } else {
                        Some(dest.to_string())
                    };
                    link_stack.push(value);
                }
                Tag::Table(alignments) => {
                    in_table = true;
                    table_alignments = alignments;
                    table_headers.clear();
                    table_rows.clear();
                    current_table_row.clear();
                    current_table_cell.clear();
                }
                Tag::TableHead => in_table_head = true,
                Tag::TableRow => current_table_row.clear(),
                Tag::TableCell => current_table_cell.clear(),
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Paragraph => {
                    if !in_table {
                        let runs = std::mem::take(&mut current_runs);
                        if in_item {
                            blocks.push(Block::ListItem {
                                depth: list_depth.saturating_sub(1),
                                runs,
                            });
                        } else {
                            blocks.push(Block::Paragraph { runs });
                        }
                    }
                }
                TagEnd::Heading(_) => {
                    let level = current_heading_level.take().unwrap_or(1);
                    blocks.push(Block::Heading {
                        level,
                        runs: std::mem::take(&mut current_runs),
                    });
                }
                TagEnd::Emphasis | TagEnd::Strong => {
                    if stack.len() > 1 {
                        stack.pop();
                    }
                }
                TagEnd::CodeBlock => {
                    in_code_fence = false;
                    blocks.push(Block::CodeFence {
                        lang: current_code_lang.take(),
                        code: std::mem::take(&mut code_fence_buf),
                    });
                }
                TagEnd::Item => in_item = false,
                TagEnd::List(_) => {
                    list_depth = list_depth.saturating_sub(1);
                }
                TagEnd::Image => {
                    in_image = false;
                    blocks.push(Block::Image {
                        path: image_dest.clone(),
                    });
                }
                TagEnd::Link => {
                    if !link_stack.is_empty() {
                        link_stack.pop();
                    }
                }
                TagEnd::TableCell => {
                    current_table_row.push(std::mem::take(&mut current_table_cell));
                }
                TagEnd::TableRow => {
                    if in_table_head {
                        table_headers = std::mem::take(&mut current_table_row);
                    } else {
                        table_rows.push(std::mem::take(&mut current_table_row));
                    }
                }
                TagEnd::TableHead => in_table_head = false,
                TagEnd::Table => {
                    in_table = false;
                    blocks.push(Block::Table {
                        alignments: std::mem::take(&mut table_alignments),
                        headers: std::mem::take(&mut table_headers),
                        rows: std::mem::take(&mut table_rows),
                    });
                }
                _ => {}
            },
            Event::Text(t) => {
                if in_code_fence {
                    code_fence_buf.push_str(&t);
                } else if !in_image {
                    let run = InlineRun {
                        text: t.to_string(),
                        style: *stack.last().unwrap_or(&InlineStyle::default()),
                        link_url: link_stack.last().cloned().flatten(),
                    };
                    if in_table {
                        current_table_cell.push(run);
                    } else {
                        current_runs.push(run);
                    }
                }
            }
            Event::Code(t) => {
                let mut style = *stack.last().unwrap_or(&InlineStyle::default());
                style.code = true;
                let run = InlineRun {
                    text: t.to_string(),
                    style,
                    link_url: link_stack.last().cloned().flatten(),
                };
                if in_table {
                    current_table_cell.push(run);
                } else {
                    current_runs.push(run);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_code_fence {
                    code_fence_buf.push('\n');
                } else {
                    let run = InlineRun::plain("\n".to_string());
                    if in_table {
                        current_table_cell.push(run);
                    } else {
                        current_runs.push(run);
                    }
                }
            }
            _ => {}
        }
    }

    SourceDoc {
        frontmatter,
        blocks,
    }
}

fn push_style(stack: &mut Vec<InlineStyle>, edit: impl FnOnce(&mut InlineStyle)) {
    let mut next = *stack.last().unwrap_or(&InlineStyle::default());
    edit(&mut next);
    stack.push(next);
}

fn split_frontmatter(markdown: &str) -> (Frontmatter, &str) {
    if !markdown.starts_with("---\n") {
        return (Frontmatter::default(), markdown);
    }

    let mut lines = markdown.lines();
    let mut consumed = 4usize;
    let _ = lines.next();
    let mut fm = Frontmatter::default();

    for line in lines {
        consumed += line.len() + 1;
        if line.trim() == "---" {
            break;
        }

        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim();
            let val = v.trim().trim_matches('"').trim_matches('\'');
            if key == "background_image" && !val.is_empty() {
                fm.background_image = Some(val.to_string());
            }
        }
    }

    let body = markdown.get(consumed..).unwrap_or(markdown);
    (fm, body)
}

pub(crate) fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

pub(crate) fn heading_size_pt(level: u8) -> f64 {
    match level {
        1 => BODY_FONT_PT * GOLDEN_RATIO * GOLDEN_RATIO * GOLDEN_RATIO * GOLDEN_RATIO,
        2 => BODY_FONT_PT * GOLDEN_RATIO * GOLDEN_RATIO,
        3 => BODY_FONT_PT * GOLDEN_RATIO,
        _ => BODY_FONT_PT,
    }
}

pub(crate) fn heading_role(level: u8) -> TextRole {
    if level == 1 {
        TextRole::Heading
    } else {
        TextRole::HeadingHeavy
    }
}
