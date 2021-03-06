#![no_std]

extern crate alloc;

pub mod decoder;

pub mod encoder;
pub(crate) mod encoder_common;
pub mod encoder_to_vec;

pub struct CompressedData<'a> {
    pub data: &'a [u8],
    pub original_size: usize,
}

#[cfg(all(unix))]
#[macro_use]
extern crate std;

#[cfg(all(unix))]
#[cfg(test)]
mod tests {
    use crate::decoder::HeatshrinkDecoder;
    use crate::encoder::HeatshrinkEncoder;

    use std::vec::Vec;

    #[test]
    fn decode_zeros() {
        let input = [0u8, 0x38];
        let mut dec = HeatshrinkDecoder::source(input.iter().cloned());

        for _ in 0..8 {
            assert_eq!(Some(0u8), dec.next());
        }
        assert_eq!(None, dec.next());
    }

    #[test]
    fn enc_dec() {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        let src = (0..100)
            .map(|_| rng.gen_range(0u8..0xff))
            .collect::<Vec<u8>>();

        println!("=src: {}", src.len());

        let enc = HeatshrinkEncoder::source(src.clone().into_iter());
        let encoded = enc.collect::<Vec<_>>();

        println!("=compressed: {}", encoded.len());

        let dec = HeatshrinkDecoder::source(encoded.into_iter());
        let decoded = dec.collect::<Vec<_>>();

        println!("=unpacked: {}", decoded.len());

        assert_eq!(src, decoded);
    }

    #[test]
    fn enc_dec_direct() {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        let src = (0..257)
            .map(|_| rng.gen_range(0u8..0xff))
            .collect::<Vec<u8>>();

        println!("=src: {}", src.len());

        let enc = HeatshrinkEncoder::source(src.clone().into_iter());
        let dec = HeatshrinkDecoder::source(enc);

        let decoded = dec.collect::<Vec<_>>();

        println!("=unpacked: {}", decoded.len());

        assert_eq!(src, decoded);
    }
}
