# darkmode-pdf

A no-options dark mode Markdown to PDF renderer that emits vector-first PDF output.

## Design constraints

- Fixed dark mode output (no theme/options flags).
- Typography:
  - Body: `FunnelSans-Light` at `10pt`
  - Body italic: `FunnelSans-LightItalic`
  - Body bold: `FunnelSans-Bold`
  - H2/H3+ headings: heaviest available `FunnelSans` in `assets/` (`ExtraBold` if present, otherwise `Bold`)
  - H1 title: `Oswald-Light`
  - Code: `JetBrainsMono-Thin` (fallback to Light if present), optically scaled to `0.8rem`
  - Heading scale: golden ratio (`1.618`) from H3 -> H2 -> H1, with H1 promoted one additional rank
- Fenced code blocks use `syntect` syntax highlighting for: Rust, Python, HTML, XML, CSS, Markdown, `ts`, TypeScript, JSON, JavaScript.
- Full-page background image support via frontmatter.
- Markdown table layout is custom (column sizing + pagination + repeated headers), not delegated to a prebuilt markdown table renderer.

### Required fonts

Place these files directly in `assets/`.

`assets/` is the preferred flat layout. `assets/fonts/` is still accepted as a
fallback for compatibility.

- `FunnelSans-Light.ttf`
- `FunnelSans-LightItalic.ttf`
- `FunnelSans-Bold.ttf`
- `Oswald-Light.ttf`
- `JetBrainsMono-Thin.ttf`

## Usage

```bash
cargo run -- input.md output.pdf
```

Exactly two args are accepted: input markdown and output PDF path.

## VS Code Extension Scaffold

A desktop VS Code extension wrapper now lives in
[`vscode-extension/`](/Users/richardklassen/Developer/darkmode-pdf/vscode-extension/README.md:1).
It is designed to bundle platform-specific `darkmode-pdf` binaries and launch
them locally from the VS Code extension host.

Current scaffold target matrix:

- `darwin-arm64`
- `win32-x64`
- `win32-arm64`
- `linux-x64`
- `linux-arm64`

The Linux path is intentionally runtime-gated to distro IDs `ubuntu` and
`arch`. VS Code packaging can target OS and architecture, but not Linux distro,
so that restriction is enforced by the extension at runtime.

## Frontmatter

Optional YAML-like frontmatter keys:

```md
---
background_image: ./media/bg-4k.jpg
---
```

`background_image` is applied as a full-page, high-resolution cover image on every page.

## Markdown examples

Code block:

~~~md
```rust
fn main() {
    println!("darkmode pdf");
}
```
~~~

Hyperlink:

```md
[Project homepage](https://example.com)
```

## Appendix: Code Samples

This appendix is intentionally render-heavy so `cargo run -- README.md out/README.pdf`
produces an out-of-the-box sample showing the supported highlighted fence types.

### HTML

```html
<article class="ring-report">
  <header>
    <h1>Saturn Systems Bulletin</h1>
    <p data-state="stable">Vector-first output confirmed.</p>
  </header>
  <section>
    <ul>
      <li>Markdown parse</li>
      <li>Dark theme render</li>
      <li>PDF write</li>
    </ul>
  </section>
</article>
```

### XML

```xml
<renderer>
  <theme mode="dark">
    <font role="body">FunnelSans-Light</font>
    <font role="heading">FunnelSans-Bold</font>
    <font role="title">Oswald-Light</font>
  </theme>
  <output format="pdf" pages="2" />
</renderer>
```

### CSS

```css
:root {
  --page-bg: #111;
  --body-fg: #aaa;
  --heading-fg: #ddd;
  --title-fg: #fff;
  --debug-bounds: #f0f;
}

.code-block {
  background: #171717;
  padding: 0.8rem;
  border: 1px solid #444b55;
}
```

### TS Alias

```ts
type Theme = {
  pageBg: string;
  text: string;
  heading: string;
};

const theme: Theme = {
  pageBg: "#111",
  text: "#aaa",
  heading: "#ddd"
};
```

### JSON

```json
{
  "name": "darkmode-pdf",
  "kind": "renderer",
  "output": "pdf",
  "debugBounds": true,
  "languages": ["rust", "python", "html", "xml", "css", "ts", "typescript", "json", "md"]
}
```

### Markdown

```md
# Sample Appendix

- body text
- heading markers
- code fences

[Reference link](https://example.com)
```

### TypeScript

```typescript
interface RenderJob {
  input: string;
  output: string;
  cwd: string;
}

function queue(job: RenderJob): string {
  return `${job.input} -> ${job.output}`;
}
```

### Rust

```rust
use std::fmt::Write as _;

#[derive(Clone, Debug)]
struct Section {
    heading: &'static str,
    lines: usize,
}

fn estimate_pages(sections: &[Section], lines_per_page: usize) -> usize {
    let total_lines: usize = sections.iter().map(|section| section.lines).sum();
    total_lines.div_ceil(lines_per_page.max(1))
}

fn render_manifest(sections: &[Section]) -> String {
    let mut out = String::new();
    for (idx, section) in sections.iter().enumerate() {
        let _ = writeln!(
            out,
            "{:02}. {} ({})",
            idx + 1,
            section.heading,
            section.lines
        );
    }
    out
}

fn main() {
    let sections = vec![
        Section {
            heading: "Frontmatter",
            lines: 12,
        },
        Section {
            heading: "Headings",
            lines: 18,
        },
        Section {
            heading: "Code Appendix",
            lines: 42,
        },
        Section {
            heading: "Tables",
            lines: 16,
        },
    ];

    let pages = estimate_pages(&sections, 32);
    let manifest = render_manifest(&sections);

    println!("estimated pages: {pages}");
    println!("{manifest}");
}
```

### Python

```python
from dataclasses import dataclass
from pathlib import Path


@dataclass
class RenderJob:
    source: Path
    output: Path
    language: str
    line_count: int


def estimate_pages(jobs: list[RenderJob], lines_per_page: int = 32) -> int:
    total_lines = sum(job.line_count for job in jobs)
    return max(1, (total_lines + lines_per_page - 1) // lines_per_page)


def build_summary(jobs: list[RenderJob]) -> str:
    rows: list[str] = []
    for index, job in enumerate(jobs, start=1):
        rows.append(
            f"{index:02d}. {job.language:<10} {job.source.name} -> {job.output.name} ({job.line_count} lines)"
        )
    return "\n".join(rows)


def main() -> None:
    jobs = [
        RenderJob(Path("README.md"), Path("out/readme.pdf"), "markdown", 120),
        RenderJob(Path("guide.md"), Path("out/guide.pdf"), "markdown", 84),
        RenderJob(Path("appendix.md"), Path("out/appendix.pdf"), "markdown", 96),
    ]

    pages = estimate_pages(jobs)
    summary = build_summary(jobs)

    print(f"estimated pages: {pages}")
    print(summary)


if __name__ == "__main__":
    main()
```

## Notes

- Text/image placement is fixed-style and deterministic.
- Backgrounds and markdown image blocks are embedded at high DPI transform settings.
- A Zulu timestamp (`YYYY-MM-DDTHH:MM:SSZ`) is rendered on the title page.

## Performance

Pdfs with hundreds of code blocks can be accelerated using IDE techniques:
+ rope text storage
+ incremental syntax highlighting
+ span caching

`#end`
