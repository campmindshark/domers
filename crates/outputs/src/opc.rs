//! Open Pixel Control wire encoder.
//!
//! Spectrum's controller expects a non-standard frame chunk without the usual
//! `0xff` prefix: `[channel][command=0][len_hi][len_lo][rgb...]`.

use domers_core::Rgb;

/// Encode one non-standard Spectrum OPC frame chunk.
///
/// # Panics
///
/// Panics if the encoded RGB payload exceeds `u16::MAX`, which is the maximum
/// length representable in the OPC frame header.
#[must_use]
pub fn encode_frame(channel: u8, pixels: &[Rgb]) -> Vec<u8> {
    let byte_len = pixels.len() * 3;
    let byte_len = u16::try_from(byte_len).expect("OPC chunk too large");
    let mut out = Vec::with_capacity(4 + usize::from(byte_len));
    out.push(channel);
    out.push(0);
    out.extend(byte_len.to_be_bytes());
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
        assert_eq!(
            encoded,
            vec![2, 0, 0, 6, 0x12, 0x34, 0x56, 0xaa, 0xbb, 0xcc]
        );
    }

    #[test]
    fn matches_extracted_csharp_opc_fixture() {
        let expected = include_bytes!(
            "../../../fixtures/spectrum-csharp/opc_packets/two_pixels_channel_2.bin"
        );
        let encoded = encode_frame(2, &[Rgb::from_u24(0x12_34_56), Rgb::from_u24(0xaa_bb_cc)]);
        assert_eq!(encoded.as_slice(), expected);
    }
}
