use proc_macro::TokenStream;


#[proc_macro]
pub fn store_coeff(cfg: TokenStream) -> TokenStream {
    cfg
}

mod tests {
    #[test]
    fn test_add() {
        assert_eq!(1 + 2, 3);
    } 
}