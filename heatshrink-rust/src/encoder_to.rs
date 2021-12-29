#![allow(non_upper_case_globals)]

use crate::encoder_common::_heatshrink_encoder;
use crate::encoder_common::{
    heatshrink_encoder_finish, heatshrink_encoder_poll, heatshrink_encoder_reset,
    heatshrink_encoder_sink,
};
use crate::encoder_common::{
    HSE_finish_res_HSER_FINISH_DONE, HSE_finish_res_HSER_FINISH_MORE, HSE_poll_res_HSER_POLL_EMPTY,
    HSE_poll_res_HSER_POLL_MORE, HSE_sink_res_HSER_SINK_OK, HEATSHRINK_STATIC_INPUT_BUFFER_SIZE,
};

pub enum Result {
    /// В выходной буфер успешно записано N байтов и запас места еще достаточен
    WritenOk(usize),
    /// В выходной буфер успешно записано N байтов и запас места меньше максимально возможного пакета
    WritenWarning(usize),

    /// В выходной буфер успешно записано N байтов и  места не хватило
    OutputFull(usize),

    // Не удалось аписать данные во входной буффер
    InputBufferFull,

    // Финализировано успешно
    Finished,
}

pub struct HeatshrinkEncoderTo<'a> {
    ctx: _heatshrink_encoder,
    writen: usize,
    dest: &'a mut [u8],
}

impl<'a> HeatshrinkEncoderTo<'a> {
    pub fn dest(buff: &'a mut [u8]) -> Self {
        let mut res = Self {
            ctx: _heatshrink_encoder::default(),
            writen: 0,
            dest: buff,
        };
        unsafe {
            heatshrink_encoder_reset(&mut res.ctx);
        }
        res
    }

    pub fn write_byte(&mut self, mut byte: u8) -> Result {
        let mut actualy_read = 0;

        match unsafe { heatshrink_encoder_sink(&mut self.ctx, &mut byte, 1, &mut actualy_read) } {
            HSE_sink_res_HSER_SINK_OK => {
                if actualy_read != 1 {
                    return Result::InputBufferFull;
                }
            }
            _ => panic!(),
        }
        self.pool()
    }

    fn pool(&mut self) -> Result {
        let dest = &mut self.dest[self.writen..];
        let mut writen = 0;
        match unsafe {
            heatshrink_encoder_poll(&mut self.ctx, dest.as_mut_ptr(), dest.len(), &mut writen)
        } {
            // данных во входном буфере не достаточно чтобы олностью заполнить предлагаемое место,
            // записано только writen байтов
            HSE_poll_res_HSER_POLL_EMPTY => {
                self.writen += writen;
                return if self.dest.len() - self.writen
                    > HEATSHRINK_STATIC_INPUT_BUFFER_SIZE as usize
                {
                    Result::WritenOk(writen)
                } else {
                    Result::WritenWarning(writen)
                };
            }

            // полностью занято все доступное место и его не хватило
            HSE_poll_res_HSER_POLL_MORE => {
                self.writen += writen;
                return Result::OutputFull(writen);
            }
            _ => panic!(),
        }
    }

    pub fn finalise(&mut self) -> Result {
        match unsafe { heatshrink_encoder_finish(&mut self.ctx) } {
            // в выходном буфере данных нет
            HSE_finish_res_HSER_FINISH_DONE => Result::Finished,
            HSE_finish_res_HSER_FINISH_MORE => self.pool(),
            _ => panic!(),
        }
    }

    #[inline]
    pub fn writen(&self) -> usize {
        self.writen
    }
}
