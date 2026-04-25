use peniko::Color as PColor;
use syntect::highlighting::Color as SynColor;

pub(crate) fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

// `colorshift` is an snorm bias in [-1, 1].
// Positive shifts warm the neutral toward red/yellow; negative shifts cool it
// toward blue. This is intentionally called "colorshift" rather than
// "temperature shift" because, for coder-facing intuition, warm/cool is the
// mental model we want while still acknowledging that blue is physically the
// higher-temperature side of the spectrum.
pub(crate) fn canonical_gray(nibble: u8, colorshift: f64) -> PColor {
    let lightness = canonical_gray_lstar(nibble);
    let shift = colorshift.clamp(-1.0, 1.0);
    let a = 6.0 * shift;
    let b = 12.0 * shift;
    let (x, y, z) = lab_to_xyz(lightness, a, b);
    let (r_lin, g_lin, b_lin) = xyz_to_linear_srgb(x, y, z);
    let r = linear_to_srgb(r_lin);
    let g = linear_to_srgb(g_lin);
    let b = linear_to_srgb(b_lin);
    PColor::from_rgba8(
        (clamp01(r) * 255.0).round() as u8,
        (clamp01(g) * 255.0).round() as u8,
        (clamp01(b) * 255.0).round() as u8,
        255,
    )
}

pub(crate) fn canonical_gray_hex(nibble: char, colorshift: f64) -> Option<PColor> {
    nibble
        .to_digit(16)
        .map(|value| canonical_gray(value as u8, colorshift))
}

pub(crate) fn blend_srgb_over(backdrop: PColor, overlay: PColor, overlay_alpha: f64) -> PColor {
    let alpha = clamp01(overlay_alpha);
    let backdrop_rgba = backdrop.to_rgba8();
    let overlay_rgba = overlay.to_rgba8();

    let blend_channel = |backdrop: u8, overlay: u8| -> u8 {
        ((f64::from(backdrop) * (1.0 - alpha)) + (f64::from(overlay) * alpha)).round() as u8
    };

    PColor::from_rgba8(
        blend_channel(backdrop_rgba.r, overlay_rgba.r),
        blend_channel(backdrop_rgba.g, overlay_rgba.g),
        blend_channel(backdrop_rgba.b, overlay_rgba.b),
        255,
    )
}

fn linear_to_srgb(value: f64) -> f64 {
    if value <= 0.003_130_8 {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

fn canonical_gray_lstar(nibble: u8) -> f64 {
    let step = nibble.min(0x0F) as f64 / 15.0;
    step * 100.0
}

fn lab_to_xyz(l: f64, a: f64, b: f64) -> (f64, f64, f64) {
    let fy = (l + 16.0) / 116.0;
    let fx = fy + (a / 500.0);
    let fz = fy - (b / 200.0);

    let xr = lab_f_inv(fx);
    let yr = lab_f_inv(fy);
    let zr = lab_f_inv(fz);

    (xr * 0.950_47, yr, zr * 1.088_83)
}

fn lab_f_inv(value: f64) -> f64 {
    let cube = value.powi(3);
    if cube > 0.008_856 {
        cube
    } else {
        (116.0 * value - 16.0) / 903.296_296_2
    }
}

fn xyz_to_linear_srgb(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    (
        3.2406 * x - 1.5372 * y - 0.4986 * z,
        -0.9689 * x + 1.8758 * y + 0.0415 * z,
        0.0557 * x - 0.2040 * y + 1.0570 * z,
    )
}

fn rgb_to_hsv(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let max = r.max(g.max(b));
    let min = r.min(g.min(b));
    let delta = max - min;
    let mut hue = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    if hue < 0.0 {
        hue += 360.0;
    }
    let sat = if max == 0.0 { 0.0 } else { delta / max };
    (hue, sat, max)
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (f64, f64, f64) {
    let c = v * s;
    let x = c * (1.0 - (((h / 60.0).rem_euclid(2.0)) - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (r1 + m, g1 + m, b1 + m)
}

pub(crate) fn syntect_color_to_peniko(color: SynColor) -> PColor {
    let r = f64::from(color.r) / 255.0;
    let g = f64::from(color.g) / 255.0;
    let b = f64::from(color.b) / 255.0;
    let a = f64::from(color.a) / 255.0;
    let (mut h, mut s, mut v) = rgb_to_hsv(r, g, b);

    if s > 0.18 && (70.0..=170.0).contains(&h) {
        h = 158.0;
        s = s.max(0.2);
        v = v.max(0.96);
    } else if s > 0.22 && (15.0..=55.0).contains(&h) {
        h = 286.0;
        s = s.max(0.2);
        v = v.max(0.82);
    }
    s = s.min(0.2);

    let (rr, gg, bb) = hsv_to_rgb(h, s, v);
    PColor::from_rgba8(
        (clamp01(rr) * 255.0).round() as u8,
        (clamp01(gg) * 255.0).round() as u8,
        (clamp01(bb) * 255.0).round() as u8,
        (clamp01(a) * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::{blend_srgb_over, canonical_gray, canonical_gray_lstar};
    use peniko::Color as PColor;

    #[test]
    fn canonical_gray_uses_even_perceptual_steps() {
        assert_eq!(canonical_gray_lstar(0x0), 0.0);
        assert_eq!(canonical_gray_lstar(0xF), 100.0);

        let c = canonical_gray_lstar(0xC);
        let d = canonical_gray_lstar(0xD);
        let e = canonical_gray_lstar(0xE);

        let delta_cd = d - c;
        let delta_de = e - d;
        assert!((delta_cd - delta_de).abs() < 1e-9);
    }

    #[test]
    fn canonical_gray_zero_shift_is_neutral() {
        let rgba = canonical_gray(0xD, 0.0).to_rgba8();
        assert_eq!(rgba.r, rgba.g);
        assert_eq!(rgba.g, rgba.b);
    }

    #[test]
    fn canonical_gray_shift_warms_and_cools() {
        let warm = canonical_gray(0xD, 1.0).to_rgba8();
        let cool = canonical_gray(0xD, -1.0).to_rgba8();

        assert!(warm.r >= warm.b);
        assert!(cool.b >= cool.r);
    }

    #[test]
    fn blend_srgb_over_interpolates_between_backdrop_and_overlay() {
        let backdrop = PColor::from_rgba8(100, 100, 100, 255);
        let overlay = PColor::from_rgba8(0, 0, 0, 255);
        let blended = blend_srgb_over(backdrop, overlay, 0.38).to_rgba8();

        assert_eq!(blended.r, 62);
        assert_eq!(blended.g, 62);
        assert_eq!(blended.b, 62);
    }
}
