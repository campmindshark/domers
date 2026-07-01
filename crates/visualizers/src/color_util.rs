use domers_core::Rgb;

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Spectrum scales and truncates RGB channels to bytes"
)]
pub(crate) fn scale_rgb_f64(color: Rgb, scale: f64) -> Rgb {
    Rgb {
        r: (f64::from(color.r) * scale).clamp(0.0, 255.0) as u8,
        g: (f64::from(color.g) * scale).clamp(0.0, 255.0) as u8,
        b: (f64::from(color.b) * scale).clamp(0.0, 255.0) as u8,
    }
}

pub(crate) fn diagnostic_colors(brightness: f32) -> [Rgb; 6] {
    [
        Rgb::from_u24(0xff_00_00).scale(brightness),
        Rgb::from_u24(0x00_ff_00).scale(brightness),
        Rgb::from_u24(0x00_00_ff).scale(brightness),
        Rgb::from_u24(0xff_ff_00).scale(brightness),
        Rgb::from_u24(0xff_00_ff).scale(brightness),
        Rgb::from_u24(0x00_ff_ff).scale(brightness),
    ]
}

pub(crate) fn white(brightness: f32) -> Rgb {
    Rgb::from_u24(0xff_ff_ff).scale(brightness)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    reason = "HSV channels are clamped before conversion to RGB bytes"
)]
pub(crate) fn hsv_to_rgb(hue: f64, saturation: f64, value: f64) -> Rgb {
    let h = hue.rem_euclid(1.0) * 6.0;
    let i = h.floor() as i32;
    let f = h - f64::from(i);
    let value = value.clamp(0.0, 1.0);
    let saturation = saturation.clamp(0.0, 1.0);
    let p = value * (1.0 - saturation);
    let q = value * (1.0 - f * saturation);
    let t = value * (1.0 - (1.0 - f) * saturation);
    let (r, g, b) = match i.rem_euclid(6) {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        _ => (value, p, q),
    };
    Rgb {
        r: (255.0 * r) as u8,
        g: (255.0 * g) as u8,
        b: (255.0 * b) as u8,
    }
}

pub(crate) fn light_paint(base: Rgb, paint: Rgb) -> Rgb {
    if paint.r.max(paint.g).max(paint.b) > base.r.max(base.g).max(base.b) {
        paint
    } else {
        base
    }
}
