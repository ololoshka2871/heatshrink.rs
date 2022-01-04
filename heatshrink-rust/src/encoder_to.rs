#![allow(non_upper_case_globals)]

use crate::encoder_common::_heatshrink_encoder;
use crate::encoder_common::{
    heatshrink_encoder_finish, heatshrink_encoder_poll, heatshrink_encoder_reset,
    heatshrink_encoder_sink,
};
use crate::encoder_common::{
    HSE_finish_res_HSER_FINISH_DONE, HSE_finish_res_HSER_FINISH_MORE, HSE_poll_res_HSER_POLL_EMPTY,
    HSE_poll_res_HSER_POLL_ERROR_MISUSE, HSE_poll_res_HSER_POLL_MORE,
    HSE_sink_res_HSER_SINK_ERROR_MISUSE, HSE_sink_res_HSER_SINK_OK, HEATSHRINK_STATIC_WINDOW_BITS,
};

pub enum Result<'a> {
    // данные успешно обработаны
    Ok(HeatshrinkEncoderTo<'a>),

    // Количество байт поступивших на вход + выходной слайс
    Done(&'a [u8]),

    // ошибка: выходной буфер кончился, финализация неуспешна
    Overflow,
}

const MINIMAL_BUFF_SIZE: usize = 1 << HEATSHRINK_STATIC_WINDOW_BITS;
/// по результатам тестов, это максимальное количество байт которое остается
/// во входном буфере после успешного poll()
const MAX_SADIMENT: usize = 15;

pub struct HeatshrinkEncoderTo<'a> {
    ctx: _heatshrink_encoder,
    dest: &'a mut [u8],
    wp: usize,
    reserved_start_pos: usize,
}

impl<'a> HeatshrinkEncoderTo<'a> {
    /// 1. Cлайс для записи должен быть размера не меньше чем MINIMAL_BWFF_SIZE
    pub fn dest(buff: &'a mut [u8]) -> Self {
        assert!(buff.len() >= MINIMAL_BUFF_SIZE);
        let mut res = Self {
            ctx: _heatshrink_encoder::default(),
            reserved_start_pos: buff.len() - MINIMAL_BUFF_SIZE,
            dest: buff,
            wp: 0,
        };
        unsafe {
            heatshrink_encoder_reset(&mut res.ctx);
        }
        res
    }

    pub fn push_bytes(mut self, mut data: &[u8]) -> Result<'a> {
        let mut writen = 0;
        match unsafe {
            heatshrink_encoder_sink(&mut self.ctx, data.as_ptr(), data.len(), &mut writen)
        } {
            HSE_sink_res_HSER_SINK_OK => {
                if writen == data.len() {
                    // все влезло, выход
                    return Result::Ok(self);
                }
            }
            HSE_sink_res_HSER_SINK_ERROR_MISUSE => {
                // Все не влезло
            }

            _ => panic!(),
        }

        data = &data[writen..];
        // Точно не влезет
        if data.len() > MINIMAL_BUFF_SIZE - MAX_SADIMENT {
            return Result::Overflow;
        }

        let normal_out_buf = &mut self.dest[self.wp..self.reserved_start_pos];
        let mut out_writen = 0;
        match unsafe {
            heatshrink_encoder_poll(
                &mut self.ctx,
                normal_out_buf.as_mut_ptr(),
                normal_out_buf.len(),
                &mut out_writen,
            )
        } {
            HSE_poll_res_HSER_POLL_EMPTY => {
                self.wp += out_writen; /* ok */

                // запись остатков
                let sink_res = unsafe {
                    heatshrink_encoder_sink(&mut self.ctx, data.as_ptr(), data.len(), &mut writen)
                };

                if sink_res != HSE_sink_res_HSER_SINK_OK {
                    return Result::Overflow;
                }

                return if self.wp == self.reserved_start_pos {
                    self.finish()
                } else {
                    Result::Ok(self)
                };
            }
            HSE_poll_res_HSER_POLL_MORE | HSE_poll_res_HSER_POLL_ERROR_MISUSE => {
                // Есть данные, которые не влезли в основной буфер, пишем их в резервную область
                self.wp += out_writen;
                let reserved_out_buf = &mut self.dest[self.wp..];
                let poll_res = unsafe {
                    heatshrink_encoder_poll(
                        &mut self.ctx,
                        reserved_out_buf.as_mut_ptr(),
                        reserved_out_buf.len(),
                        &mut out_writen,
                    )
                };
                match poll_res {
                    HSE_poll_res_HSER_POLL_EMPTY => {
                        // Обновляем позицию для следующей зписи
                        self.wp += out_writen;

                        // запись остатков
                        let sink_res = unsafe {
                            heatshrink_encoder_sink(
                                &mut self.ctx,
                                data.as_ptr(),
                                data.len(),
                                &mut writen,
                            )
                        };

                        if sink_res != HSE_sink_res_HSER_SINK_OK {
                            return Result::Overflow;
                        }
                        return self.finish();
                    }
                    HSE_poll_res_HSER_POLL_MORE => return Result::Overflow,
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    /// Любые данные, просто побайтно скармливаются упаковщику
    pub fn push<T: Copy>(self, data: T) -> Result<'a> {
        let data = unsafe {
            core::slice::from_raw_parts(
                &data as *const _ as *const u8,
                core::mem::size_of_val(&data),
            )
        };

        self.push_bytes(data)
    }

    pub fn finish(mut self) -> Result<'a> {
        let result = unsafe { heatshrink_encoder_finish(&mut self.ctx) };
        match result {
            HSE_finish_res_HSER_FINISH_MORE => {
                let out_buf = &mut self.dest[self.wp..];
                let mut out_writen = 0;
                match unsafe {
                    heatshrink_encoder_poll(
                        &mut self.ctx,
                        out_buf.as_mut_ptr(),
                        out_buf.len(),
                        &mut out_writen,
                    )
                } {
                    // Все успешно обработано, все влезло в выходной буффер
                    HSE_poll_res_HSER_POLL_EMPTY => {
                        self.wp += out_writen;
                        Result::Done(&self.dest[..self.wp])
                    }
                    // Финализировано неудачно, остаток данных не влез в указанный буфер
                    // Записанные данные неконсистентны, остается только выбросить все в мусор
                    HSE_poll_res_HSER_POLL_MORE => Result::Overflow,
                    // ошибка
                    _ => panic!(),
                }
            }
            HSE_finish_res_HSER_FINISH_DONE => Result::Done(&self.dest[..self.wp]),
            _ => panic!(),
        }
    }
}

#[cfg(all(unix))]
#[cfg(test)]
mod tests {
    extern crate alloc;
    use core::mem;

    use alloc::vec::Vec;

    use crate::{
        decoder::HeatshrinkDecoder, encoder::HeatshrinkEncoder, encoder_to::HeatshrinkEncoderTo,
    };

    #[test]
    fn encode_to_basic() {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let mut dest = [0u8; 4096];
        let mut src = Vec::new();
        let mut in_count = 0usize;

        let mut encoder = HeatshrinkEncoderTo::dest(&mut dest[..]);

        let res = loop {
            let v = rng.gen_range(0..u32::MAX);
            src.push(v);

            match encoder.push(v) {
                crate::encoder_to::Result::Ok(e) => {
                    encoder = e;
                    in_count += mem::size_of::<u32>();
                }
                crate::encoder_to::Result::Done(result) => {
                    in_count += mem::size_of::<u32>();
                    println!(
                        "Packed {} input bytes to {} compressed",
                        in_count,
                        result.len()
                    );
                    break result;
                }
                crate::encoder_to::Result::Overflow => panic!("overrun"),
            }
        };

        let normaly_encoded =
            HeatshrinkEncoder::source(src.iter().map(|i| u32::to_le_bytes(*i)).flatten())
                .collect::<Vec<_>>();
        let decoder = HeatshrinkDecoder::source(res.iter().cloned());

        let r = decoder.collect::<Vec<_>>();
        assert_eq!(res, normaly_encoded.as_slice());
        assert_eq!(r.len(), in_count);
        let r = r
            .chunks(mem::size_of::<u32>())
            .map(|c| {
                let mut v = [0; mem::size_of::<u32>()];
                v.copy_from_slice(c);
                u32::from_le_bytes(v)
            })
            .collect::<Vec<_>>();
        assert_eq!(r, src);
    }
}
