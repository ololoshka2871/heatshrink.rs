
mod tests {
    use heatshrink_rust::decoder::HeatshrinkDecoder;
    use heatshrink_rust_macro::{packed_bytes, packed_file};

    #[test]
    fn test_packed_string() {
        static PACKED_STRING: &[u8; 16] = packed_bytes!(b"my test string");
        let decoder = HeatshrinkDecoder::source(PACKED_STRING.iter().cloned());

        assert_eq!(decoder.collect::<Vec<_>>(), b"my test string");
    }

    #[test]
    fn test_packed_file() {
        static FILE_DATA: &[u8; 749] = include_bytes!("test1.rs");
        static FILE_PACKED_DATA: &[u8; 441] = packed_file!("tests/test1.rs");

        let decoder = HeatshrinkDecoder::source(FILE_PACKED_DATA.iter().cloned());

        assert_eq!(decoder.collect::<Vec<_>>().as_slice(), FILE_DATA);
    }
}