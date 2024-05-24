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

    let app_configs_name = format_ident!("{name}AppConfigs");
    let preparers_name = format_ident!("{name}InstancePreparers");
    let state_name = format_ident!("{name}InstanceState");

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
    let ConfiguredApp = quote!(#factors_path::ConfiguredApp);
    let TypeId = quote!(::std::any::TypeId);

    Ok(quote! {
        impl #name {
            pub fn init(
                &mut self,
                mut linker: Option<&mut #wasmtime::component::Linker<#state_name>>,
                mut module_linker: Option<&mut #wasmtime::Linker<#state_name>>,
            ) -> #Result<()> {
                #(
                    #Factor::init::<Self>(
                        &mut self.#factor_names,
                        #factors_path::InitContext::<Self, #factor_types>::new(
                            linker.as_deref_mut(),
                            module_linker.as_deref_mut(),
                            |state| &mut state.#factor_names,
                        )
                    )?;
                )*
                Ok(())
            }

            pub fn configure_app(&self, app: #factors_path::App) -> #Result<#ConfiguredApp<Self>> {
                let mut app_configs = #app_configs_name {
                    #( #factor_names: None, )*
                };
                #(
                    app_configs.#factor_names = Some(
                        #Factor::configure_app(
                            &self.#factor_names,
                            &app,
                            #factors_path::ConfigureAppContext::<Self>::new(&app_configs),
                        )?
                    );
                )*
                Ok(#ConfiguredApp::new(app, app_configs))
            }

            pub fn build_store_data(&self, configured_app: &#ConfiguredApp<Self>, component_id: &str) -> #Result<#state_name> {
                let app_component = configured_app.app().get_component(component_id).ok_or_else(|| {
                    #factors_path::Error::msg(format!("unknown component {component_id:?}"))
                })?;
                let mut preparers = #preparers_name {
                    #( #factor_names: None, )*
                };
                #(
                    preparers.#factor_names = Some(
                        #factors_path::FactorInstancePreparer::<#factor_types>::new::<#name>(
                            &self.#factor_names,
                            &app_component,
                            #factors_path::PrepareContext::new(configured_app, &mut preparers),
                        )?
                    );
                )*
                Ok(#state_name {
                    #(
                        #factor_names: #factors_path::FactorInstancePreparer::<#factor_types>::prepare(
                            preparers.#factor_names.unwrap(),
                        )?,
                    )*
                })
            }

        }

        impl #factors_path::SpinFactors for #name {
            type AppConfigs = #app_configs_name;
            type InstancePreparers = #preparers_name;
            type InstanceState = #state_name;

            unsafe fn instance_preparer_offset<T: #Factor>() -> Option<usize> {
                let type_id = #TypeId::of::<T>();
                #(
                    if type_id == #TypeId::of::<#factor_types>() {
                        return Some(std::mem::offset_of!(Self::InstancePreparers, #factor_names));
                    }
                )*
                None
            }

            unsafe fn instance_state_offset<T: #Factor>() -> Option<usize> {
                let type_id = #TypeId::of::<T>();
                #(
                    if type_id == #TypeId::of::<#factor_types>() {
                        return Some(std::mem::offset_of!(Self::InstanceState, #factor_names));
                    }
                )*
                None

            }

            fn app_config<T: #Factor>(app_configs: &Self::AppConfigs) -> Option<&T::AppConfig> {
                let type_id = #TypeId::of::<T>();
                #(
                    if type_id == #TypeId::of::<#factor_types>() {
                        return Some(unsafe { std::mem::transmute(&app_configs.#factor_names) });
                    }
                )*
                None
            }
        }

        #vis struct #app_configs_name {
            #(
                pub #factor_names: Option<<#factor_types as #Factor>::AppConfig>,
            )*
        }

        #vis struct #preparers_name {
            #(
                pub #factor_names: Option<<#factor_types as #Factor>::InstancePreparer>,
            )*
        }

        #vis struct #state_name {
            #(
                pub #factor_names: <#factor_types as #Factor>::InstanceState,
            )*
        }
    })
}
