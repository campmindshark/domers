//! Open Pixel Control wire encoder.
//!
//! Spectrum's controller expects a non-standard frame chunk without the usual
//! `0xff` prefix: `[channel][command=0][len_hi][len_lo][rgb...]`.

use domers_core::Rgb;

/// Encode one non-standard Spectrum OPC frame chunk.
#[must_use]
pub fn encode_frame(channel: u8, pixels: &[Rgb]) -> Vec<u8> {
    let byte_len = pixels.len() * 3;
    assert!(byte_len <= u16::MAX as usize, "OPC chunk too large");
    let mut out = Vec::with_capacity(4 + byte_len);
    out.push(channel);
    out.push(0);
    out.push(((byte_len >> 8) & 0xff) as u8);
    out.push((byte_len & 0xff) as u8);
    for pixel in pixels {
        out.push(pixel.r);
        out.push(pixel.g);
        out.push(pixel.b);
    }
    out
}

#[cfg(test)]
mod tests {
    use domers_core::Rgb;

    use super::encode_frame;

    #[test]
    fn encodes_spectrum_nonstandard_header_without_magic_prefix() {
        let encoded = encode_frame(2, &[Rgb::from_u24(0x12_34_56), Rgb::from_u24(0xaa_bb_cc)]);
        assert_eq!(encoded, vec![2, 0, 0, 6, 0x12, 0x34, 0x56, 0xaa, 0xbb, 0xcc]);
    }
}
