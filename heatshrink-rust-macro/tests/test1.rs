
mod tests {
    use heatshrink_rust::decoder::HeatshrinkDecoder;
    use heatshrink_rust_macro::{packed_string, packed_bytes, packed_file};

    #[test]
    fn test_packed_string() {
        static PACKED_STRING: &[u8; 28] = packed_string!("Тестовая строка");
        let decoder = HeatshrinkDecoder::source(PACKED_STRING.iter().cloned());

        assert_eq!(decoder.collect::<Vec<_>>(), "Тестовая строка".as_bytes());
    }

    #[test]
    fn test_packed_bytes() {
        static PACKED_STRING: &[u8; 16] = packed_bytes!(b"my test string");
        let decoder = HeatshrinkDecoder::source(PACKED_STRING.iter().cloned());

        assert_eq!(decoder.collect::<Vec<_>>(), b"my test string");
    }

    #[test]
    fn test_packed_file() {
        static FILE_DATA: &[u8; 1361] = include_bytes!("../src/lib.rs");
        static FILE_PACKED_DATA: &[u8; 801] = packed_file!("heatshrink-rust-macro/src/lib.rs");

        let decoder = HeatshrinkDecoder::source(FILE_PACKED_DATA.iter().cloned());

        assert_eq!(decoder.collect::<Vec<_>>().as_slice(), FILE_DATA);
    }
}