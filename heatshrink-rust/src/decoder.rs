#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]
#![allow(deprecated)]

include!("bindings/bindings-decoder.rs");

impl Default for _heatshrink_decoder {
    fn default() -> _heatshrink_decoder {
        unsafe { core::mem::uninitialized() }
    }
}

pub struct HeatshrinkDecoder<T>
where
    T: Iterator<Item = u8>,
{
    ctx: _heatshrink_decoder,
    finished: bool,
    src: T,
}

impl<T> HeatshrinkDecoder<T>
where
    T: Iterator<Item = u8>,
{
    pub fn source(src: T) -> Self {
        let mut res = Self {
            ctx: _heatshrink_decoder::default(),
            finished: false,
            src,
        };
        unsafe {
            heatshrink_decoder_reset(&mut res.ctx);
        }
        res
    }
}

impl<T> Iterator for HeatshrinkDecoder<T>
where
    T: Iterator<Item = u8>,
{
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut outbuf: u8 = 0;
            let mut actualy_read: usize = 0;
            let res = unsafe {
                heatshrink_decoder_poll(&mut self.ctx, &mut outbuf, 1, &mut actualy_read)
            };
            match res {
                HSDR_sink_res_HSER_POLL_EMPTY => {
                    if actualy_read == 0 {
                        if self.finished {
                            return None;
                        }
                    } else {
                        return Some(outbuf);
                    }
                }
                HSDR_sink_res_HSER_POLL_MORE => {
                    // ok
                    if actualy_read == 1 {
                        return Some(outbuf);
                    } else {
                        panic!(
                            "heatshrink_encoder_poll: Requested read 1 byte, but {} got",
                            actualy_read
                        );
                    }
                }
                _ => panic!(),
            }

            // need more data
            if let Some(mut b) = self.src.next() {
                let mut actualy_read: usize = 0;
                let mut res =
                    unsafe { heatshrink_decoder_sink(&mut self.ctx, &mut b, 1, &mut actualy_read) };
                match res {
                    HSD_sink_res_HSDR_SINK_OK => {} // ok
                    _ => panic!(),
                }
            } else {
                // try finalise
                self.finished = true;
                let res = unsafe { heatshrink_decoder_finish(&mut self.ctx) };
                match res {
                    HSDR_finish_res_HSER_FINISH_DONE => return None, // ok
                    HSDR_finish_res_HSER_FINISH_MORE => {} // there is data in encoder buff
                    _ => panic!(),
                }
            }
        }
    }
}

#[cfg(all(unix))]
#[cfg(test)]
mod tests {
    use crate::decoder::HeatshrinkDecoder;

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
}
