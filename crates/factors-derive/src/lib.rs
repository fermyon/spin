use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Error};

#[proc_macro_derive(SpinFactors)]
pub fn derive_factors(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let expanded = expand_factors(&input).unwrap_or_else(|err| err.into_compile_error());

    #[cfg(feature = "expander")]
    let expanded = expander::Expander::new("factors")
        .write_to_out_dir(expanded)
        .unwrap();

    expanded.into()
}

#[allow(non_snake_case)]
fn expand_factors(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let vis = &input.vis;

    let builders_name = format_ident!("{name}Builders");
    let data_name = format_ident!("{name}Data");

    if !input.generics.params.is_empty() {
        return Err(Error::new_spanned(
            input,
            "cannot derive Factors for generic structs",
        ));
    }

    // Get struct fields
    let fields = match &input.data {
        Data::Struct(struct_data) => &struct_data.fields,
        _ => {
            return Err(Error::new_spanned(
                input,
                "can only derive Factors for structs",
            ))
        }
    };
    let mut factor_names = Vec::with_capacity(fields.len());
    let mut factor_types = Vec::with_capacity(fields.len());
    for field in fields.iter() {
        factor_names.push(
            field
                .ident
                .as_ref()
                .ok_or_else(|| Error::new_spanned(input, "tuple structs are not supported"))?,
        );
        factor_types.push(&field.ty);
    }

    let factors_crate = format_ident!("spin_factors");
    let factors_path = quote!(::#factors_crate);
    let Factor = quote!(#factors_path::Factor);
    let Result = quote!(#factors_path::Result);
    let wasmtime = quote!(#factors_path::wasmtime);
    let TypeId = quote!(::std::any::TypeId);

    Ok(quote! {
        impl #name {
            pub fn init(
                &mut self,
                linker: &mut #wasmtime::component::Linker<#data_name>
            ) -> #Result<()> {
                #(
                    self.#factor_names.init(
                        #factors_path::InitContext::<Self, #factor_types>::new(
                            linker,
                            |data| &mut data.#factor_names,
                        )
                    )?;
                )*
                Ok(())
            }

            pub fn module_init(
                &mut self,
                linker: &mut #wasmtime::Linker<#data_name>
            ) -> #Result<()> {
                #(
                    self.#factor_names.module_init::<Self>(
                        #factors_path::ModuleInitContext::<Self, #factor_types>::new(
                            linker,
                            |data| &mut data.#factor_names,
                        )
                    )?;
                )*
                Ok(())
            }

            pub fn build_data(&self) -> #Result<#data_name> {
                let mut builders = #builders_name {
                    #( #factor_names: None, )*
                };
                #(
                    builders.#factor_names = Some(
                        #factors_path::FactorBuilder::<#factor_types>::prepare::<#name>(
                            &self.#factor_names,
                            #factors_path::PrepareContext::new(&mut builders),
                        )?
                    );
                )*
                Ok(#data_name {
                    #(
                        #factor_names: #factors_path::FactorBuilder::<#factor_types>::build(
                            builders.#factor_names.unwrap()
                        )?,
                    )*
                })
            }

        }

        impl #factors_path::SpinFactors for #name {
            type Builders = #builders_name;
            type Data = #data_name;

            unsafe fn factor_builder_offset<T: #Factor>() -> Option<usize> {
                let type_id = #TypeId::of::<T>();
                #(
                    if type_id == #TypeId::of::<#factor_types>() {
                        return Some(std::mem::offset_of!(Self::Builders, #factor_names));
                    }
                )*
                None
            }

            unsafe fn factor_data_offset<T: #Factor>() -> Option<usize> {
                let type_id = #TypeId::of::<T>();
                #(
                    if type_id == #TypeId::of::<#factor_types>() {
                        return Some(std::mem::offset_of!(Self::Data, #factor_names));
                    }
                )*
                None

            }
        }

        #vis struct #builders_name {
            #(
                pub #factor_names: Option<<#factor_types as #Factor>::Builder>,
            )*
        }

        #vis struct #data_name {
            #(
                pub #factor_names: <#factor_types as #Factor>::Data,
            )*
        }
    })
}
