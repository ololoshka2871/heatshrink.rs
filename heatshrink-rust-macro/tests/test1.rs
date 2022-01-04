
mod tests {
    use heatshrink_rust::decoder::HeatshrinkDecoder;
    use heatshrink_rust::CompressedData;
    use heatshrink_rust_macro::{packed_string, packed_bytes, packed_file};

    #[test]
    fn test_packed_string() {
        static PACKED_STRING: CompressedData = packed_string!("Тестовая строка");
        let decoder = HeatshrinkDecoder::source(PACKED_STRING.data.iter().cloned());

        assert_eq!(decoder.collect::<Vec<_>>(), "Тестовая строка".as_bytes());
    }

    #[test]
    fn test_packed_bytes() {
        static PACKED_STRING: CompressedData= packed_bytes!(b"my test string");
        let decoder = HeatshrinkDecoder::source(PACKED_STRING.data.iter().cloned());

        assert_eq!(decoder.collect::<Vec<_>>(), b"my test string");
    }

    #[test]
    fn test_packed_file() {
        static FILE_DATA: &[u8; 1617] = include_bytes!("../src/lib.rs");
        static FILE_PACKED_DATA: CompressedData = packed_file!("heatshrink-rust-macro/src/lib.rs");

        let decoder = HeatshrinkDecoder::source(FILE_PACKED_DATA.data.iter().cloned());

        assert_eq!(decoder.collect::<Vec<_>>().as_slice(), FILE_DATA);
    }
}