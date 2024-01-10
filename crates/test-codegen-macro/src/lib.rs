use heck::*;
use proc_macro::TokenStream;
use std::{env, path::PathBuf};

/// This macro generates the `#[test]` functions for the runtime tests.
#[proc_macro]
pub fn codegen_tests(_input: TokenStream) -> TokenStream {
    let mut tests = Vec::new();
    let tests_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/runtime-tests");
    let tests_path_string = tests_path
        .to_str()
        .expect("CARGO_MANIFEST_DIR is not valid utf8")
        .to_owned();
    for entry in std::fs::read_dir(tests_path).expect("failed to read tests directory") {
        let entry = entry.expect("error reading test directory entry");
        let test = entry.path();

        if entry.file_type().unwrap().is_dir() {
            let requires_services = entry.path().join("services").exists();

            let name = test.file_stem().unwrap().to_str().unwrap();
            let ident = quote::format_ident!("{}", name.to_snake_case());
            let feature_attribute = if requires_services {
                quote::quote!(#[cfg(feature = "e2e-tests")])
            } else {
                quote::quote!()
            };
            // Generate the following code:
            // ```rust
            // #[test]
            // fn outbound_mysql() {
            //     run("outbound-mysql")
            // }
            // ```
            tests.push(quote::quote! {
                #[test]
                #feature_attribute
                fn #ident() {
                    run(::std::path::PathBuf::from(#tests_path_string).join(#name))
                }
            });
        }
    }
    (quote::quote!(#(#tests)*)).into()
}
