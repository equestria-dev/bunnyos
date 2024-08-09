extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::ItemFn;

#[proc_macro_attribute]
pub fn bunny_entry(_args: TokenStream, input: TokenStream) -> TokenStream {
    let parsed = syn::parse::<ItemFn>(input.clone()).unwrap();
    let name = parsed.sig.ident;
    let input: proc_macro2::TokenStream = input.into();

    let out = quote! {
        use uefi::prelude::*;
        #input

        #[entry] fn __bunny_main(_image: Handle, mut system_table: SystemTable<Boot>) -> Status {
            unsafe { bstd::init(system_table, _image); }
            #name();
            Status::SUCCESS
        }
    };

    out.into()
}
