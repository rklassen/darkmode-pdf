use crate::model::{InlineRun, MeasureFonts, TextRole};
use crate::viewmodel::text::{
    measure_inline_run_advance_mm, measure_line_run_advance_mm, measure_run_width_mm,
};

pub(crate) fn measure_line_width_mm(
    fonts: &MeasureFonts,
    line: &[InlineRun],
    size_pt: f64,
    role: TextRole,
    force_bold: bool,
) -> f64 {
    line.iter()
        .enumerate()
        .map(|(idx, _)| measure_line_run_advance_mm(fonts, line, idx, size_pt, role, force_bold))
        .sum()
}

pub(crate) fn measure_runs_max_line_width_mm(
    fonts: &MeasureFonts,
    runs: &[InlineRun],
    size_pt: f64,
    role: TextRole,
    force_bold: bool,
) -> f64 {
    let mut max_w: f64 = 0.0;
    let mut current_line: Vec<InlineRun> = Vec::new();

    for run in runs {
        for chunk in run.text.split_inclusive('\n') {
            let text = chunk.strip_suffix('\n').unwrap_or(chunk);
            if !text.is_empty() {
                current_line.push(InlineRun {
                    text: text.to_string(),
                    style: run.style,
                    link_url: run.link_url.clone(),
                });
            }
            if chunk.ends_with('\n') {
                max_w = max_w.max(measure_line_width_mm(
                    fonts,
                    &current_line,
                    size_pt,
                    role,
                    force_bold,
                ));
                current_line.clear();
            }
        }
    }

    max_w.max(measure_line_width_mm(
        fonts,
        &current_line,
        size_pt,
        role,
        force_bold,
    ))
}

pub(crate) fn measure_runs_longest_token_mm(
    fonts: &MeasureFonts,
    runs: &[InlineRun],
    size_pt: f64,
    role: TextRole,
    force_bold: bool,
) -> f64 {
    let mut max_w: f64 = 0.0;
    for run in runs {
        if run.style.code {
            max_w = max_w.max(measure_inline_run_advance_mm(
                fonts, &run.text, run.style, size_pt, role, force_bold,
            ));
            continue;
        }
        let tokenized = run.text.replace('\n', " ");
        for token in tokenized.split_whitespace() {
            max_w = max_w.max(measure_run_width_mm(
                fonts, token, run.style, size_pt, role, force_bold,
            ));
        }
    }
    max_w
}
