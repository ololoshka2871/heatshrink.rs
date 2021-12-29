#![no_std]

pub mod decoder;

pub mod encoder;
pub(crate) mod encoder_common;
pub mod encoder_to;

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
    use crate::encoder_to::HeatshrinkEncoderTo;

    use std::vec::Vec;

    #[test]
    fn encode_static_data() {
        static DATA: &[u8; 19] = b"s;djfdlsdj\x00\0128sdfs";
        let _ = HeatshrinkEncoder::source(DATA.iter().cloned()).collect::<Vec<_>>();
    }

    #[test]
    fn encode_zeros() {
        let zeros = [0u8; 8];
        let mut enc = HeatshrinkEncoder::source(zeros.iter().cloned());

        //result
        assert_eq!(Some(0x0), enc.next());
        assert_eq!(Some(0x38), enc.next());
        assert_eq!(None, enc.next());
    }

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

        let src = (0..100)
            .map(|_| rng.gen_range(0u8..0xff))
            .collect::<Vec<u8>>();

        println!("=src: {}", src.len());

        let enc = HeatshrinkEncoder::source(src.clone().into_iter());
        let dec = HeatshrinkDecoder::source(enc);

        let decoded = dec.collect::<Vec<_>>();

        println!("=unpacked: {}", decoded.len());

        assert_eq!(src, decoded);
    }

    #[test]
    fn enc_to() {
        use rand::Rng;

        const DEST_LEN: usize = 64;
        let mut rng = rand::thread_rng();

        let mut dest = vec![0u8; DEST_LEN];
        let mut enc = HeatshrinkEncoderTo::dest(dest.as_mut_slice());
        let mut i = 0;

        loop {
            match enc.write_byte(rng.gen_range(0u8..0xff)) {
                crate::encoder_to::Result::WritenOk(_) => {
                    /* ok */
                    i += 1;
                }
                crate::encoder_to::Result::WritenWarning(_) => break,
                _ => panic!(),
            }
        }
        match enc.finalise() {
            crate::encoder_to::Result::WritenWarning(_) | crate::encoder_to::Result::Finished => {
                /* ok */
            }
            _ => panic!(),
        }

        assert!(enc.writen() <= DEST_LEN);
        println!("=pack: {} to {}", i, enc.writen());
    }
}
