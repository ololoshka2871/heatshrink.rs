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
                HSDR_sink_res_HSER_POLL_ERROR_NULL => panic!("Nullptr!"), /* NULL argument */
                HSDR_sink_res_HSER_POLL_ERROR_MISUSE => panic!(),         /* API misuse */
            }

            // need more data
            if let Some(mut b) = self.src.next() {
                let mut actualy_read: usize = 0;
                let mut res =
                    unsafe { heatshrink_decoder_sink(&mut self.ctx, &mut b, 1, &mut actualy_read) };
                match res {
                    HSD_sink_res_HSDR_SINK_OK => {}                // ok
                    HSD_sink_res_HSDR_SINK_ERROR_NULL => panic!(), // buffer full
                    HSD_sink_res_HSER_SINK_ERROR_NULL => panic!("Nullptr!"),
                    N => panic!("Unknown result heatshrink_decoder_sink: {}", N),
                }
            } else {
                // try finalise
                self.finished = true;
                let res = unsafe { heatshrink_decoder_finish(&mut self.ctx) };
                match res {
                    HSDR_finish_res_HSER_FINISH_DONE => return None, // ok
                    HSDR_finish_res_HSER_FINISH_ERROR_NULL => panic!("Nullptr!"),
                    HSDR_finish_res_HSER_FINISH_MORE => {} // there is data in encoder buff
                    N => panic!("Unknown result heatshrink_decoder_finish: {}", N),
                }
            }
        }
    }
}
