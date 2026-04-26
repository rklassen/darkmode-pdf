use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn source_font_root() -> PathBuf {
    let flat = repo_root().join("assets");
    if flat.join("FunnelSans-Light.ttf").exists() || flat.join("FunnelSans-Light.otf").exists() {
        return flat;
    }
    repo_root().join("assets/fonts")
}

fn setup_hermetic_workspace() -> Result<TempDir, Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    let src_fonts = source_font_root();
    let dst_fonts = root.join("assets/fonts");
    fs::create_dir_all(&dst_fonts)?;

    for entry in fs::read_dir(src_fonts)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_file() {
            let name = p.file_name().expect("font filename");
            fs::copy(&p, dst_fonts.join(name))?;
        }
    }

    fs::create_dir_all(root.join("docs/media"))?;
    fs::create_dir_all(root.join("out"))?;
    Ok(tmp)
}

fn write_solid_png(path: &Path, rgb: [u8; 3]) -> Result<(), Box<dyn std::error::Error>> {
    let mut img = image::RgbImage::new(64, 64);
    for px in img.pixels_mut() {
        *px = image::Rgb(rgb);
    }
    img.save(path)?;
    Ok(())
}

fn assert_pdf_was_written(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    assert!(bytes.starts_with(b"%PDF"), "expected PDF header");
    assert!(bytes.len() > 1024, "PDF output is unexpectedly tiny");
    Ok(())
}

fn assert_pdf_contains_uri(path: &Path, uri: &str) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    let needle = uri.as_bytes();
    assert!(
        bytes.windows(needle.len()).any(|window| window == needle),
        "expected PDF to contain URI annotation target"
    );
    Ok(())
}

#[test]
fn hermetic_relative_paths_render_markdown() -> Result<(), Box<dyn std::error::Error>> {
    let ws = setup_hermetic_workspace()?;
    let root = ws.path();

    write_solid_png(&root.join("docs/media/bg.png"), [20, 20, 20])?;
    write_solid_png(&root.join("docs/media/inline.png"), [90, 90, 120])?;

    let md = r#"---
background_image: ./media/bg.png
---

# Hermetic Relative Render

Body paragraph with **bold**, _italic_, and `inline code`.

![inline image](./media/inline.png)

```rust
fn main() {
    println!("hermetic");
}
```
"#;

    fs::write(root.join("docs/input.md"), md)?;

    let status = Command::new(env!("CARGO_BIN_EXE_darkmode-pdf"))
        .current_dir(root)
        .arg("docs/input.md")
        .arg("out/output.pdf")
        .status()?;

    assert!(status.success(), "renderer command failed");
    assert_pdf_was_written(&root.join("out/output.pdf"))?;
    Ok(())
}

#[test]
fn hermetic_relative_paths_render_multilang_code_fences() -> Result<(), Box<dyn std::error::Error>>
{
    let ws = setup_hermetic_workspace()?;
    let root = ws.path();

    let md = r#"# Syntect Language Smoke

```rust
fn add(a: i32, b: i32) -> i32 { a + b }
```

```python
def add(a, b):
    return a + b
```

```html
<div class=\"planet\">Saturn</div>
```

```xml
<planet name=\"Saturn\" />
```

```css
:root { --ring-color: #c9b037; }
.planet { color: var(--ring-color); }
```

```md
# markdown

- ring
```

```ts
const ringCount: number = 7;
```

```typescript
const ringCount: number = 7;
```

```json
{"planet":"Saturn","rings":7}
```

```js
const saturn = { rings: 7 };
```
"#;

    fs::write(root.join("docs/input.md"), md)?;

    let status = Command::new(env!("CARGO_BIN_EXE_darkmode-pdf"))
        .current_dir(root)
        .arg("docs/input.md")
        .arg("out/multilang.pdf")
        .status()?;

    assert!(status.success(), "renderer command failed");
    assert_pdf_was_written(&root.join("out/multilang.pdf"))?;
    Ok(())
}

#[test]
fn hermetic_render_embeds_hyperlink_annotations() -> Result<(), Box<dyn std::error::Error>> {
    let ws = setup_hermetic_workspace()?;
    let root = ws.path();

    let md = r#"# Link Smoke

See [OpenAI](https://openai.com/) for details.
"#;

    fs::write(root.join("docs/input.md"), md)?;

    let status = Command::new(env!("CARGO_BIN_EXE_darkmode-pdf"))
        .current_dir(root)
        .arg("docs/input.md")
        .arg("out/links.pdf")
        .status()?;

    assert!(status.success(), "renderer command failed");
    let out = root.join("out/links.pdf");
    assert_pdf_was_written(&out)?;
    assert_pdf_contains_uri(&out, "https://openai.com/")?;
    Ok(())
}
