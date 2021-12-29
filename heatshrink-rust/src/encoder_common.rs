#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]
#![allow(deprecated)]

include!("bindings/bindings-encoder.rs");

impl Default for _heatshrink_encoder {
    fn default() -> Self {
        unsafe { core::mem::uninitialized() }
    }
}

impl _heatshrink_encoder {
    pub(crate) const fn input_buffer_size() -> usize {
        1 << HEATSHRINK_STATIC_WINDOW_BITS
    }

    pub(crate) fn input_size(&self) -> u16 {
        self.input_size
    }
}

#[cfg(all(unix))]
#[cfg(test)]
mod tests {
    extern crate alloc;

    use core::ops::Add;
    use core::{mem, slice};

    use alloc::vec::Vec;

    use crate::decoder::HeatshrinkDecoder;
    use crate::encoder_common::{
        heatshrink_encoder_finish, heatshrink_encoder_poll, heatshrink_encoder_reset,
        heatshrink_encoder_sink,
    };

    use crate::encoder_common::_heatshrink_encoder;
    use crate::encoder_common::{
        HSE_finish_res_HSER_FINISH_MORE, HSE_poll_res_HSER_POLL_EMPTY, HSE_poll_res_HSER_POLL_MORE,
        HSE_sink_res_HSER_SINK_OK, HEATSHRINK_STATIC_WINDOW_BITS,
    };

    #[test]
    fn test_fill_input_and_pool() {
        let mut encoder = _heatshrink_encoder::default();

        unsafe { heatshrink_encoder_reset(&mut encoder) };

        // входной буфер упаковщика должен быть заполнен полностью или передан признак остановки
        // иначе pool() ни чего не будет возвращать
        let src = (0.._heatshrink_encoder::input_buffer_size())
            .map(|n| (n & 0xff) as u8)
            .collect::<Vec<u8>>();

        // заливаем полный входной буфер
        let mut writen = 0;
        let result =
            unsafe { heatshrink_encoder_sink(&mut encoder, src.as_ptr(), src.len(), &mut writen) };
        assert_eq!(result, HSE_sink_res_HSER_SINK_OK);
        assert_eq!(src.len(), writen);

        // готовим место под результат в 2 раза больше чем исходник
        let mut out_buf = Vec::with_capacity(_heatshrink_encoder::input_buffer_size() * 2);
        let mut out_size = 0;

        // пытаемся сжать
        let result = unsafe {
            heatshrink_encoder_poll(
                &mut encoder,
                out_buf.as_mut_ptr(),
                out_buf.capacity(),
                &mut out_size,
            )
        };
        // должно сказать, что в выходном буфере больше нет данных
        assert_eq!(result, HSE_poll_res_HSER_POLL_EMPTY);
        // должно быть записано меньше или равно размеру буфера
        assert!(out_size <= out_buf.capacity());
        // говорим вектору, что у него уже записано стьлько-то байт
        unsafe { out_buf.set_len(out_size) };
        // в энкодере остаются байты, но видимо это мусор
        assert!(encoder.input_size() > 0);

        // Подаем признак конца данных
        let result = unsafe { heatshrink_encoder_finish(&mut encoder) };
        // должен сказать, что есть данные на выход
        assert_eq!(result, HSE_finish_res_HSER_FINISH_MORE);
        // еще раз pool-им
        let prev_len = out_buf.len();
        let dest = unsafe {
            slice::from_raw_parts_mut(
                out_buf.as_mut_ptr().add(prev_len),
                out_buf.capacity() - prev_len,
            )
        };
        let result = unsafe {
            heatshrink_encoder_poll(&mut encoder, dest.as_mut_ptr(), dest.len(), &mut out_size)
        };
        // должно сказать, что в выходном буфере больше нет данных
        assert_eq!(result, HSE_poll_res_HSER_POLL_EMPTY);
        // не должно записать больше, чем есть места
        assert!(out_size <= dest.len());
        // говорим вектору что у него новый размер
        unsafe { out_buf.set_len(prev_len + out_size) };
        // после heatshrink_encoder_finish() это значение - мусор
        assert!(encoder.input_size() > 0);

        println!(
            "=compresed: {} ({} body + {} finalise) /{}/",
            out_buf.len(),
            prev_len,
            out_buf.len() - prev_len,
            encoder.input_size()
        );

        // проверка корректности упаковки, должно совпадать с исходником
        let decoder = HeatshrinkDecoder::source(out_buf.into_iter());
        assert_eq!(src, decoder.collect::<Vec<_>>());
    }

    #[test]
    fn test_fill_input_not_full_pool() {
        let mut encoder = _heatshrink_encoder::default();

        unsafe { heatshrink_encoder_reset(&mut encoder) };

        //половина входного буфера
        let src = (0.._heatshrink_encoder::input_buffer_size() / 2)
            .map(|n| (n & 0xff) as u8)
            .collect::<Vec<u8>>();

        // заливаем входной буфер
        let mut writen = 0;
        let result =
            unsafe { heatshrink_encoder_sink(&mut encoder, src.as_ptr(), src.len(), &mut writen) };
        assert_eq!(result, HSE_sink_res_HSER_SINK_OK);
        assert_eq!(src.len(), writen);

        // готовим место под результат в 2 раза больше чем исходник
        let mut out_buf = Vec::with_capacity(_heatshrink_encoder::input_buffer_size() * 2);
        let mut out_size = 0;

        // пытаемся сжать
        let result = unsafe {
            heatshrink_encoder_poll(
                &mut encoder,
                out_buf.as_mut_ptr(),
                out_buf.capacity(),
                &mut out_size,
            )
        };
        // должно сказать, что в выходном буфере больше нет данных
        assert_eq!(result, HSE_poll_res_HSER_POLL_EMPTY);
        // должно быть записано меньше или равно размеру буфера
        assert_eq!(out_size, 0);
        // в энкодере все записанные байты
        assert_eq!(encoder.input_size() as usize, src.len());

        // Подаем признак конца данных
        let result = unsafe { heatshrink_encoder_finish(&mut encoder) };
        // должен сказать, что есть данные на выход
        assert_eq!(result, HSE_finish_res_HSER_FINISH_MORE);
        // еще раз pool-им
        let result = unsafe {
            heatshrink_encoder_poll(
                &mut encoder,
                out_buf.as_mut_ptr(),
                out_buf.capacity(),
                &mut out_size,
            )
        };
        // должно сказать, что в выходном буфере больше нет данных
        assert_eq!(result, HSE_poll_res_HSER_POLL_EMPTY);
        // не должно записать больше, чем есть места
        assert!(out_size <= out_buf.capacity());
        // говорим вектору что у него новый размер
        unsafe { out_buf.set_len(out_size) };
        // после heatshrink_encoder_finish() это значение - мусор
        assert!(encoder.input_size() > 0);

        println!("=compresed: {} /{}/", out_buf.len(), encoder.input_size());

        // проверка корректности упаковки, должно совпадать с исходником
        let decoder = HeatshrinkDecoder::source(out_buf.into_iter());
        assert_eq!(src, decoder.collect::<Vec<_>>());
    }

    #[test]
    fn test_fill_pool_fill() {
        let mut encoder = _heatshrink_encoder::default();

        unsafe { heatshrink_encoder_reset(&mut encoder) };

        // входной буфер упаковщика должен быть заполнен полностью или передан признак остановки
        // иначе pool() ни чего не будет возвращать
        let src = (0.._heatshrink_encoder::input_buffer_size())
            .map(|n| (n & 0xff) as u8)
            .collect::<Vec<u8>>();

        // заливаем полный входной буфер
        let mut writen = 0;
        let result =
            unsafe { heatshrink_encoder_sink(&mut encoder, src.as_ptr(), src.len(), &mut writen) };
        assert_eq!(result, HSE_sink_res_HSER_SINK_OK);
        assert_eq!(src.len(), writen);

        // готовим место под результат в 2 раза больше чем исходник
        let mut out_buf = Vec::with_capacity(_heatshrink_encoder::input_buffer_size() * 2);
        let mut out_size = 0;

        // пытаемся сжать
        let result = unsafe {
            heatshrink_encoder_poll(
                &mut encoder,
                out_buf.as_mut_ptr(),
                out_buf.capacity(),
                &mut out_size,
            )
        };
        // должно сказать, что в выходном буфере больше нет данных
        assert_eq!(result, HSE_poll_res_HSER_POLL_EMPTY);
        // должно быть записано меньше или равно размеру буфера
        assert!(out_size <= out_buf.capacity());
        // говорим вектору, что у него уже записано стьлько-то байт
        unsafe { out_buf.set_len(out_size) };
        // в энкодере остаются байты, но видимо это мусор
        assert!(encoder.input_size() > 0);

        let result =
            unsafe { heatshrink_encoder_sink(&mut encoder, src.as_ptr(), src.len(), &mut writen) };
        // Должно завершиться успехом
        assert_eq!(result, HSE_sink_res_HSER_SINK_OK);
        // все не должно влезть
        assert!(src.len() > writen);
    }

    #[test]
    fn test_discover_overflow() {
        use rand::Rng;
        use std::collections::HashMap;

        let mut rng = rand::thread_rng();

        let mut runner = || {
            let mut encoder = _heatshrink_encoder::default();

            unsafe { heatshrink_encoder_reset(&mut encoder) };

            // олный входной буфер рандомных чисел
            let src = (0.._heatshrink_encoder::input_buffer_size())
                .map(|_| rng.gen_range(0u8..0xff))
                .collect::<Vec<u8>>();

            // заливаем полный входной буфер
            let mut writen = 0;
            let result = unsafe {
                heatshrink_encoder_sink(&mut encoder, src.as_ptr(), src.len(), &mut writen)
            };

            // готовим место под результат в 2 раза больше чем исходник
            let mut out_buf = Vec::with_capacity(_heatshrink_encoder::input_buffer_size() * 2);
            let mut out_size = 0;

            // пытаемся сжать
            let result = unsafe {
                heatshrink_encoder_poll(
                    &mut encoder,
                    out_buf.as_mut_ptr(),
                    out_buf.capacity(),
                    &mut out_size,
                )
            };
            unsafe { out_buf.set_len(out_size) };
            // в энкодере остаются байты, но видимо это мусор
            let overflow = encoder.input_size();

            // Подаем признак конца данных
            let result = unsafe { heatshrink_encoder_finish(&mut encoder) };
            // должен сказать, что есть данные на выход
            assert_eq!(result, HSE_finish_res_HSER_FINISH_MORE);
            // еще раз pool-им
            let prev_len = out_buf.len();
            let dest = unsafe {
                slice::from_raw_parts_mut(
                    out_buf.as_mut_ptr().add(prev_len),
                    out_buf.capacity() - prev_len,
                )
            };
            let result = unsafe {
                heatshrink_encoder_poll(&mut encoder, dest.as_mut_ptr(), dest.len(), &mut out_size)
            };
            unsafe { out_buf.set_len(prev_len + out_size) };

            let result_size = out_buf.len();

            // проверка корректности упаковки, должно совпадать с исходником
            let decoder = HeatshrinkDecoder::source(out_buf.into_iter());
            assert_eq!(src, decoder.collect::<Vec<_>>());

            (
                result_size - _heatshrink_encoder::input_buffer_size(),
                overflow,
            )
        };

        let mut resultmap = HashMap::new();

        for _ in 0..1000 {
            let result = runner();
            let entry = resultmap.entry(result).or_insert(0u32);
            *entry += 1;
        }

        println!("{:?}", resultmap);
    }

    // этот тест показывает, что в любом случае во входном буфере остается <= 15 байт
    // при каждом удачном pool'е
    // так же известно, что pool не делается, пока не заполнен входной буфер на любом промежуточном шаге.
    // Т.Е. если первый раз он был заполнен до упора, сделан pool, осталось около 15 байт,
    // бесполезно вызывать pool пока буфер снова не наполнится.
    #[test]
    fn test_discover_compress_random() {
        use rand::Rng;
        use std::collections::HashMap;

        let mut rng = rand::thread_rng();

        let mut runner = |n| {
            let mut encoder = _heatshrink_encoder::default();

            unsafe { heatshrink_encoder_reset(&mut encoder) };

            // рандомные данные
            let src = (0..n)
                .map(|_| rng.gen_range(0u8..0xff))
                .collect::<Vec<u8>>();
            // готовим место под результат в 2 раза больше чем исходник
            let mut out_buf = vec![0u8; src.len() * 2];

            let mut src_slice = src.as_slice();
            let mut out_slice = out_buf.as_mut_slice();
            let mut out_writen_total = 0;
            loop {
                // записываем сколько влезет
                let mut writen = 0;
                let result = unsafe {
                    heatshrink_encoder_sink(
                        &mut encoder,
                        src_slice.as_ptr(),
                        src_slice.len(),
                        &mut writen,
                    )
                };
                assert_eq!(result, HSE_sink_res_HSER_SINK_OK);

                src_slice = &src_slice[writen..];
                let finish = src_slice.len() == 0;

                let input_size = if finish {
                    // записан последний блок
                    let result = unsafe { heatshrink_encoder_finish(&mut encoder) };
                    // должен сказать, что есть данные на выход
                    assert_eq!(result, HSE_finish_res_HSER_FINISH_MORE);

                    encoder.input_size() as usize - writen
                } else {
                    0
                };

                let mut out_writen = 0;
                // пытаемся сжать
                let result = unsafe {
                    heatshrink_encoder_poll(
                        &mut encoder,
                        out_slice.as_mut_ptr(),
                        out_slice.len(),
                        &mut out_writen,
                    )
                };
                assert_eq!(result, HSE_poll_res_HSER_POLL_EMPTY);
                out_slice = &mut out_slice[out_writen..];
                out_writen_total += out_writen;

                if finish {
                    return (out_writen, input_size);
                }
            }
        };

        let mut resultmap = HashMap::new();

        for _ in 0..1000 {
            let result = runner(4096);
            let entry = resultmap.entry(result).or_insert(0u32);
            *entry += 1;
        }

        println!("{:?}", resultmap);
    }

    // Этот тест показывает, что
    // 1. берем буфер произвольного размера Buff
    // 2. набиваем его по стандартной схеме: [sync (до упора) -> pool()] -> Buff
    // 3. когда pool() вернет HSE_poll_res_HSER_POLL_MORE создаем буфер OvfB размера _heatshrink_encoder::input_buffer_size()
    // 4. Финализируемся heatshrink_encoder_finish()
    // 5. pull() -> OvfB и все **ТОЧНО ВЛЕЗЕТ** проверено на рандомных данных, неииспользовано
    //
    // применяя к записи: 4К - вся страница
    // 4К - _heatshrink_encoder::input_buffer_size() основная часть
    // _heatshrink_encoder::input_buffer_size() - довесок
    // остается не более 1% неиспользовано с рандомными данными
    #[test]
    fn test_fill_buffer() {
        use rand::Rng;
        use std::collections::HashMap;

        let mut rng = rand::thread_rng();

        let mut runner = |s| {
            assert!(s >= _heatshrink_encoder::input_buffer_size() * 2);
            assert!(s & 0b11 == 0); // кратно 4 байтам
            let mut encoder = _heatshrink_encoder::default();

            unsafe { heatshrink_encoder_reset(&mut encoder) };

            // готовим место под результат размера s - размер_входного_буфера
            let mut out_buf = Vec::with_capacity(s);
            // это место для обычных данных
            // оставляем _heatshrink_encoder::input_buffer_size() для записи перебора
            out_buf.resize(s - _heatshrink_encoder::input_buffer_size(), 0);
            let mut out_slice = out_buf.as_mut_slice();

            let mut src_writen = 0;
            let mut out_writen_total = 0;

            loop {
                while (encoder.input_size() as usize) < _heatshrink_encoder::input_buffer_size() {
                    let v = rng.gen_range(0..u32::MAX);
                    let mut writen = 0;
                    let result = unsafe {
                        heatshrink_encoder_sink(
                            &mut encoder,
                            &v as *const _ as *const u8,
                            mem::size_of::<u32>(),
                            &mut writen,
                        )
                    };
                    assert_eq!(result, HSE_sink_res_HSER_SINK_OK);
                    src_writen += writen;
                }

                let mut out_writen = 0;
                // пытаемся сжать
                let result = unsafe {
                    heatshrink_encoder_poll(
                        &mut encoder,
                        out_slice.as_mut_ptr(),
                        out_slice.len(),
                        &mut out_writen,
                    )
                };

                match result {
                    HSE_poll_res_HSER_POLL_EMPTY => {
                        out_slice = &mut out_slice[out_writen..];
                        out_writen_total += out_writen;
                    }
                    HSE_poll_res_HSER_POLL_MORE => {
                        out_writen_total += out_writen;

                        // В этом случае значение encoder.input_size() не показательно
                        // let was_in_input = encoder.input_size();
                        let mut ovf_buf = unsafe {
                            slice::from_raw_parts_mut(
                                out_buf.as_mut_ptr().add(out_writen_total),
                                _heatshrink_encoder::input_buffer_size(),
                            )
                        };

                        let result = unsafe { heatshrink_encoder_finish(&mut encoder) };
                        // должен сказать, что есть данные на выход
                        assert_eq!(result, HSE_finish_res_HSER_FINISH_MORE);

                        let result = unsafe {
                            heatshrink_encoder_poll(
                                &mut encoder,
                                ovf_buf.as_mut_ptr(),
                                ovf_buf.len(),
                                &mut out_writen,
                            )
                        };
                        assert_eq!(result, HSE_poll_res_HSER_POLL_EMPTY);
                        out_writen_total += out_writen;
                        unsafe { out_buf.set_len(out_writen_total) };

                        // % * 100
                        return ((s - out_writen_total) as f32 / s as f32 * 10000.0).round() as u32;
                    }
                    _ => panic!(),
                }
            }
        };

        let mut resultmap = HashMap::new();

        for _ in 0..1000 {
            let result = runner(4096);
            let entry = resultmap
                .entry(format!("{:.2}%", result as f32 / 100.0))
                .or_insert(0u32);
            *entry += 1;
        }

        println!("{:?}", resultmap);
    }
}
