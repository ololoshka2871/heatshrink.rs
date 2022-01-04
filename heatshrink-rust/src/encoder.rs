#![allow(non_upper_case_globals)]

use crate::encoder_common::_heatshrink_encoder;
use crate::encoder_common::{
    heatshrink_encoder_finish, heatshrink_encoder_poll, heatshrink_encoder_reset,
    heatshrink_encoder_sink,
};
use crate::encoder_common::{
    HSE_finish_res_HSER_FINISH_DONE, HSE_finish_res_HSER_FINISH_MORE, HSE_poll_res_HSER_POLL_EMPTY,
    HSE_poll_res_HSER_POLL_MORE, HSE_sink_res_HSER_SINK_OK,
};

pub struct HeatshrinkEncoder<T>
where
    T: Iterator<Item = u8>,
{
    ctx: _heatshrink_encoder,
    finished: bool,

    // Поскольку это трейт а не объект нужно чтобы ссылка жила не меньше чем сама структура
    src: T,
}

impl<'a, T> HeatshrinkEncoder<T>
where
    T: Iterator<Item = u8>,
{
    pub fn source(src: T) -> Self {
        let mut res = Self {
            ctx: _heatshrink_encoder::default(),
            finished: false,
            src, // то же что src: src
        };
        unsafe {
            heatshrink_encoder_reset(&mut res.ctx);
        }
        res
    }
}

impl<T> Iterator for HeatshrinkEncoder<T>
where
    T: Iterator<Item = u8>,
{
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut outbuf: u8 = 0;
            let mut actualy_read: usize = 0;
            let res = unsafe {
                heatshrink_encoder_poll(&mut self.ctx, &mut outbuf, 1, &mut actualy_read)
            };
            match res {
                HSE_poll_res_HSER_POLL_EMPTY => {
                    if actualy_read == 0 {
                        if self.finished {
                            return None;
                        }
                    } else {
                        return Some(outbuf);
                    }
                }
                HSE_poll_res_HSER_POLL_MORE => {
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
                let res =
                    unsafe { heatshrink_encoder_sink(&mut self.ctx, &mut b, 1, &mut actualy_read) };
                match res {
                    HSE_sink_res_HSER_SINK_OK => {} // ok
                    _ => panic!(),
                }
            } else {
                // try finalise
                self.finished = true;
                let res = unsafe { heatshrink_encoder_finish(&mut self.ctx) };
                match res {
                    HSE_finish_res_HSER_FINISH_DONE => return None, // ok
                    HSE_finish_res_HSER_FINISH_MORE => {}           // there is data in encoder buff
                    _ => panic!(),
                }
            }
        }
    }
}

#[cfg(all(unix))]
#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::vec::Vec;

    use crate::encoder::HeatshrinkEncoder;

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
}
