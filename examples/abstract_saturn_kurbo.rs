use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use kurbo::{BezPath, Point};

#[derive(Clone, Copy)]
struct Rgb8 {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Clone)]
struct GradientStop {
    offset: f64,
    color: Rgb8,
}

#[derive(Clone)]
struct ElementGradient {
    id: &'static str,
    stops: Vec<GradientStop>,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        bail!(
            "Usage: cargo run --example abstract_saturn_kurbo -- <nasa_saturn_photo> <output.svg>"
        );
    }

    let nasa_photo = Path::new(&args[1]);
    let output_svg = Path::new(&args[2]);

    // "Inference" here means sampling palette statistics from the NASA image at runtime.
    let palette = infer_palette_from_nasa_photo(nasa_photo)?;
    let (planet, back_ring, front_ring) = abstract_saturn_paths(Point::new(420.0, 300.0), 120.0);
    let gradients = gradients_from_palette(&palette);
    let svg = build_saturn_svg(&planet, &back_ring, &front_ring, &gradients);

    fs::write(output_svg, svg)
        .with_context(|| format!("failed to write SVG: {}", output_svg.display()))?;

    println!("wrote {}", output_svg.display());
    Ok(())
}

fn infer_palette_from_nasa_photo(path: &Path) -> Result<Vec<Rgb8>> {
    let img = image::open(path)
        .with_context(|| format!("failed to open NASA photo: {}", path.display()))?
        .to_rgb8();

    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        bail!("image has zero dimensions");
    }

    let mut samples = Vec::new();
    let gx = 16;
    let gy = 16;

    for yi in 0..gy {
        for xi in 0..gx {
            let x = ((xi as f32 + 0.5) / gx as f32 * (w.saturating_sub(1)) as f32) as u32;
            let y = ((yi as f32 + 0.5) / gy as f32 * (h.saturating_sub(1)) as f32) as u32;
            let px = img.get_pixel(x, y);
            samples.push(Rgb8 {
                r: px[0],
                g: px[1],
                b: px[2],
            });
        }
    }

    samples.sort_by_key(|c| luminance(*c));
    let dark = samples[samples.len() / 8];
    let mid = samples[samples.len() / 2];
    let bright = samples[samples.len() * 7 / 8];
    let accent = most_saturated(&samples);

    Ok(vec![dark, mid, bright, accent])
}

fn most_saturated(samples: &[Rgb8]) -> Rgb8 {
    let mut best = samples[0];
    let mut best_score = saturation(best);
    for s in samples.iter().copied().skip(1) {
        let sc = saturation(s);
        if sc > best_score {
            best = s;
            best_score = sc;
        }
    }
    best
}

fn saturation(c: Rgb8) -> i32 {
    let max = c.r.max(c.g).max(c.b) as i32;
    let min = c.r.min(c.g).min(c.b) as i32;
    max - min
}

fn luminance(c: Rgb8) -> u32 {
    (u32::from(c.r) * 2126 + u32::from(c.g) * 7152 + u32::from(c.b) * 722) / 10_000
}

fn gradients_from_palette(palette: &[Rgb8]) -> Vec<ElementGradient> {
    let dark = palette[0];
    let mid = palette[1];
    let bright = palette[2];
    let accent = palette[3];

    vec![
        ElementGradient {
            id: "planetGrad",
            stops: vec![
                GradientStop {
                    offset: 0.0,
                    color: bright,
                },
                GradientStop {
                    offset: 0.58,
                    color: mid,
                },
                GradientStop {
                    offset: 1.0,
                    color: dark,
                },
            ],
        },
        ElementGradient {
            id: "ringGrad",
            stops: vec![
                GradientStop {
                    offset: 0.0,
                    color: brighten(accent, 18),
                },
                GradientStop {
                    offset: 0.5,
                    color: mid,
                },
                GradientStop {
                    offset: 1.0,
                    color: dark,
                },
            ],
        },
    ]
}

fn brighten(c: Rgb8, amount: i16) -> Rgb8 {
    let clamp = |v: i16| -> u8 { v.clamp(0, 255) as u8 };
    Rgb8 {
        r: clamp(i16::from(c.r) + amount),
        g: clamp(i16::from(c.g) + amount),
        b: clamp(i16::from(c.b) + amount),
    }
}

fn abstract_saturn_paths(center: Point, radius: f64) -> (BezPath, BezPath, BezPath) {
    let planet = bezier_circle(center, radius);

    let mut back_ring = BezPath::new();
    back_ring.move_to((center.x - radius * 1.75, center.y + radius * 0.06));
    back_ring.curve_to(
        (center.x - radius * 1.05, center.y + radius * 0.88),
        (center.x + radius * 1.05, center.y + radius * 0.88),
        (center.x + radius * 1.75, center.y + radius * 0.06),
    );
    back_ring.curve_to(
        (center.x + radius * 1.00, center.y - radius * 0.18),
        (center.x - radius * 1.00, center.y - radius * 0.18),
        (center.x - radius * 1.75, center.y + radius * 0.06),
    );
    back_ring.close_path();

    let mut front_ring = BezPath::new();
    front_ring.move_to((center.x - radius * 1.75, center.y + radius * 0.06));
    front_ring.curve_to(
        (center.x - radius * 1.10, center.y - radius * 0.52),
        (center.x + radius * 1.10, center.y - radius * 0.52),
        (center.x + radius * 1.75, center.y + radius * 0.06),
    );
    front_ring.curve_to(
        (center.x + radius * 1.00, center.y + radius * 0.24),
        (center.x - radius * 1.00, center.y + radius * 0.24),
        (center.x - radius * 1.75, center.y + radius * 0.06),
    );
    front_ring.close_path();

    (planet, back_ring, front_ring)
}

fn bezier_circle(center: Point, radius: f64) -> BezPath {
    let k = 0.552_284_749_8 * radius;
    let cx = center.x;
    let cy = center.y;

    let mut p = BezPath::new();
    p.move_to((cx + radius, cy));
    p.curve_to(
        (cx + radius, cy + k),
        (cx + k, cy + radius),
        (cx, cy + radius),
    );
    p.curve_to(
        (cx - k, cy + radius),
        (cx - radius, cy + k),
        (cx - radius, cy),
    );
    p.curve_to(
        (cx - radius, cy - k),
        (cx - k, cy - radius),
        (cx, cy - radius),
    );
    p.curve_to(
        (cx + k, cy - radius),
        (cx + radius, cy - k),
        (cx + radius, cy),
    );
    p.close_path();
    p
}

fn build_saturn_svg(
    planet: &BezPath,
    back_ring: &BezPath,
    front_ring: &BezPath,
    gradients: &[ElementGradient],
) -> String {
    let defs = gradients
        .iter()
        .map(|g| {
            let stops = g
                .stops
                .iter()
                .map(|s| {
                    format!(
                        "<stop offset=\"{:.0}%\" stop-color=\"{}\"/>",
                        s.offset * 100.0,
                        rgb_hex(s.color)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            let gradient_tag = if g.id == "planetGrad" {
                format!(
                    "<radialGradient id=\"{}\" cx=\"45%\" cy=\"40%\" r=\"70%\">{}</radialGradient>",
                    g.id, stops
                )
            } else {
                format!(
                    "<linearGradient id=\"{}\" x1=\"0%\" y1=\"0%\" x2=\"100%\" y2=\"100%\">{}</linearGradient>",
                    g.id, stops
                )
            };
            gradient_tag
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="840" height="600" viewBox="0 0 840 600">
  <rect x="0" y="0" width="840" height="600" fill="#0D1117"/>
  <defs>{defs}</defs>
  <path d="{back}" fill="url(#ringGrad)" opacity="0.48"/>
  <path d="{planet}" fill="url(#planetGrad)"/>
  <path d="{front}" fill="url(#ringGrad)" opacity="0.84"/>
</svg>"##,
        defs = defs,
        back = back_ring.to_svg(),
        planet = planet.to_svg(),
        front = front_ring.to_svg()
    )
}

fn rgb_hex(c: Rgb8) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b)
}
