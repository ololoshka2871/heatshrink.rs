#![allow(non_upper_case_globals)]

use alloc::vec::Vec;

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

pub enum Result {
    // данные успешно обработаны
    Ok(HeatshrinkEncoderToVec),

    // Количество байт поступивших на вход + выходной слайс
    Done(Vec<u8>),

    // ошибка: выходной буфер кончился, финализация неуспешна
    Overflow,
}

const MINIMAL_BUFF_SIZE: usize = 1 << HEATSHRINK_STATIC_WINDOW_BITS;
/// по результатам тестов, это максимальное количество байт которое остается
/// во входном буфере после успешного poll()
const MAX_SADIMENT: usize = 15;

pub struct HeatshrinkEncoderToVec {
    ctx: _heatshrink_encoder,
    dest: Vec<u8>,
    wp: usize,
    reserved_start_pos: usize,
}

impl HeatshrinkEncoderToVec {
    /// 1. Cлайс для записи должен быть капасити не меньше чем MINIMAL_BWFF_SIZE
    pub fn dest(mut dest: Vec<u8>, offset: usize) -> Self {
        assert!(dest.capacity() >= MINIMAL_BUFF_SIZE);

        // tamporary change vector size to it's max capasity
        unsafe { dest.set_len(dest.capacity()) };
        let mut res = Self {
            ctx: _heatshrink_encoder::default(),
            reserved_start_pos: dest.len() - MINIMAL_BUFF_SIZE,
            dest,
            wp: offset,
        };
        unsafe {
            heatshrink_encoder_reset(&mut res.ctx);
        }
        res
    }

    pub fn push_bytes(mut self, mut data: &[u8]) -> Result {
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
    pub fn push<T: Copy>(self, data: T) -> Result {
        let data = unsafe {
            core::slice::from_raw_parts(
                &data as *const _ as *const u8,
                core::mem::size_of_val(&data),
            )
        };

        self.push_bytes(data)
    }

    pub fn finish(mut self) -> Result {
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
                        unsafe { self.dest.set_len(self.wp) };
                        Result::Done(self.dest)
                    }
                    // Финализировано неудачно, остаток данных не влез в указанный буфер
                    // Записанные данные неконсистентны, остается только выбросить все в мусор
                    HSE_poll_res_HSER_POLL_MORE => Result::Overflow,
                    // ошибка
                    _ => panic!(),
                }
            }
            HSE_finish_res_HSER_FINISH_DONE => {
                unsafe { self.dest.set_len(self.wp) };
                Result::Done(self.dest)
            }
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
        decoder::HeatshrinkDecoder, encoder::HeatshrinkEncoder,
        encoder_to_vec::HeatshrinkEncoderToVec,
    };

    #[test]
    fn encode_to_basic() {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let dest = Vec::with_capacity(4096);
        let mut src = Vec::new();
        let mut in_count = 0usize;

        let mut encoder = HeatshrinkEncoderToVec::dest(dest, 0);

        let res = loop {
            let v = rng.gen_range(0..u32::MAX);
            src.push(v);

            match encoder.push(v) {
                crate::encoder_to_vec::Result::Ok(e) => {
                    encoder = e;
                    in_count += mem::size_of::<u32>();
                }
                crate::encoder_to_vec::Result::Done(result) => {
                    in_count += mem::size_of::<u32>();
                    println!(
                        "Packed {} input bytes to {} compressed",
                        in_count,
                        result.len()
                    );
                    break result;
                }
                crate::encoder_to_vec::Result::Overflow => panic!("overrun"),
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

    #[test]
    fn encode_to_with_offset() {
        use rand::Rng;

        const OFFSET: usize = 16;

        let mut rng = rand::thread_rng();
        let mut dest = Vec::with_capacity(4096);
        let mut src = Vec::new();
        let mut in_count = 0usize;

        for i in 0..OFFSET {
            dest.push(i as u8);
        }

        let mut encoder = HeatshrinkEncoderToVec::dest(dest, OFFSET);

        let res = loop {
            let v = rng.gen_range(0..u32::MAX);
            src.push(v);

            match encoder.push(v) {
                crate::encoder_to_vec::Result::Ok(e) => {
                    encoder = e;
                    in_count += mem::size_of::<u32>();
                }
                crate::encoder_to_vec::Result::Done(result) => {
                    in_count += mem::size_of::<u32>();
                    println!(
                        "Packed {} input bytes to {} compressed",
                        in_count,
                        result.len()
                    );
                    break result;
                }
                crate::encoder_to_vec::Result::Overflow => panic!("overrun"),
            }
        };

        assert_eq!(res[0..OFFSET], (0..OFFSET).map(|i| i as u8).collect::<Vec<_>>());
    }


    #[test]
    fn encode_interrupt() {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let dest = Vec::with_capacity(4096);
        let mut src = Vec::new();
        let mut in_count = 0usize;

        let mut encoder = HeatshrinkEncoderToVec::dest(dest, 0);

        let res = loop {
            let v = rng.gen_range(0..u32::MAX);
            src.push(v);

            match encoder.push(v) {
                crate::encoder_to_vec::Result::Ok(e) => {
                    encoder = e;
                    in_count += mem::size_of::<u32>();
                }
                crate::encoder_to_vec::Result::Done(result) => {
                    in_count += mem::size_of::<u32>();
                    println!(
                        "Packed {} input bytes to {} compressed",
                        in_count,
                        result.len()
                    );
                    break result;
                }
                crate::encoder_to_vec::Result::Overflow => panic!("overrun"),
            }

            if src.len() > 1500 / 4 {
                match encoder.finish() {
                    crate::encoder_to_vec::Result::Done(result) => {
                        println!(
                            "Packed interrupt {} input bytes to {} compressed",
                            in_count,
                            result.len()
                        );
                        break result;
                    }
                    _ => panic!(),
                }
            }
        };

        let decoder = HeatshrinkDecoder::source(res.iter().cloned());

        let r = decoder.collect::<Vec<_>>();
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
