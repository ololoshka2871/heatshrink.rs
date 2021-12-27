use std::path::PathBuf;

use heatshrink_rust::encoder::HeatshrinkEncoder;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitByteStr, LitStr};

fn puck<T: Iterator<Item = u8>>(iter: T, original_size: usize) -> TokenStream {
    let encoder = HeatshrinkEncoder::source(iter);

    let compressed = encoder.collect::<Vec<_>>();
    // Эта штука правильно составит стайс и правильно укажет тип элементов - u8.
    // Итерирование по образцу #(#_var_),* — the character before the asterisk is used as a separator
    quote! {
        CompressedData {
            data: &[#(#compressed),*],
            original_size: #original_size,
        }
    }
    .into()
}

#[proc_macro]
pub fn packed_string(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as LitStr).value();
    let len = input.len();
    puck(input.bytes().into_iter(), len)
}

#[proc_macro]
pub fn packed_bytes(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as LitByteStr).value();
    let len = input.len();
    puck(input.into_iter(), len)
}

#[proc_macro]
pub fn packed_file(file: TokenStream) -> TokenStream {
    let infile = parse_macro_input!(file as LitStr).value();
    let path = PathBuf::from(infile);
    if !path.exists() {
        panic!(
            "file '{:?}' in '{:?}' not found",
            path,
            std::env::current_dir().unwrap()
        );
    }

    let data = std::fs::read(path).unwrap();
    let len = data.len();
    puck(data.into_iter(), len)
}
