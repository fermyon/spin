use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Error};

#[proc_macro_derive(RuntimeFactors)]
pub fn derive_factors(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let expanded = expand_factors(&input).unwrap_or_else(|err| err.into_compile_error());

    #[cfg(feature = "expander")]
    let expanded = if let Some(dest_dir) = std::env::var_os("SPIN_FACTORS_DERIVE_EXPAND_DIR") {
        expander::Expander::new("factors")
            .write_to(expanded, std::path::Path::new(&dest_dir))
            .unwrap()
    } else {
        expanded
    };

    expanded.into()
}

#[allow(non_snake_case)]
fn expand_factors(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let vis = &input.vis;

    let app_state_name = format_ident!("{name}AppState");
    let builders_name = format_ident!("{name}InstanceBuilders");
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

    let Any = quote!(::std::any::Any);
    let Send = quote!(::std::marker::Send);
    let TypeId = quote!(::std::any::TypeId);
    let factors_crate = format_ident!("spin_factors");
    let factors_path = quote!(::#factors_crate);
    let wasmtime = quote!(#factors_path::wasmtime);
    let Result = quote!(#factors_path::Result);
    let Factor = quote!(#factors_path::Factor);
    let ConfiguredApp = quote!(#factors_path::ConfiguredApp);
    let RuntimeConfigTracker = quote!(#factors_path::__internal::RuntimeConfigTracker);
    let FactorInstanceBuilder = quote!(#factors_path::FactorInstanceBuilder);

    Ok(quote! {
        impl #factors_path::RuntimeFactors for #name {
            type AppState = #app_state_name;
            type InstanceBuilders = #builders_name;
            type InstanceState = #state_name;

            #[allow(clippy::needless_option_as_deref)]
            fn init(
                &mut self,
                linker: &mut #wasmtime::component::Linker<#state_name>,
            ) -> #Result<()> {
                #(
                    #Factor::init::<Self>(
                        &mut self.#factor_names,
                        #factors_path::InitContext::<Self, #factor_types>::new(
                            linker,
                            |state| &mut state.#factor_names,
                        )
                    )?;
                )*
                Ok(())
            }

            fn configure_app(
                &self,
                app: #factors_path::App,
                runtime_config: impl #factors_path::RuntimeConfigSource
            ) -> #Result<#ConfiguredApp<Self>> {
                let mut app_state = #app_state_name {
                    #( #factor_names: None, )*
                };
                let mut runtime_config_tracker = #RuntimeConfigTracker::new(runtime_config);
                #(
                    app_state.#factor_names = Some(
                        #Factor::configure_app(
                            &self.#factor_names,
                            #factors_path::ConfigureAppContext::<Self, #factor_types>::new(
                                &app,
                                &app_state,
                                &mut runtime_config_tracker,
                            )?,
                        )?
                    );
                )*
                runtime_config_tracker.validate_all_keys_used()?;
                Ok(#ConfiguredApp::new(app, app_state))
            }

            fn build_store_data(
                &self, configured_app: &#ConfiguredApp<Self>,
                component_id: &str,
            ) -> #Result<Self::InstanceState> {
                let app_component = configured_app.app().get_component(component_id).ok_or_else(|| {
                    #wasmtime::Error::msg("unknown component")
                })?;
                let mut builders = #builders_name {
                    #( #factor_names: None, )*
                };
                #(
                    builders.#factor_names = Some(
                        #Factor::prepare::<Self>(
                            &self.#factor_names,
                            #factors_path::PrepareContext::new(
                                configured_app.app_state::<#factor_types>().unwrap(),
                                &app_component,
                            ),
                            &mut #factors_path::InstanceBuilders::new(&mut builders),
                        )?
                    );
                )*
                Ok(#state_name {
                    #(
                        #factor_names: #FactorInstanceBuilder::build(builders.#factor_names.unwrap())?,
                    )*
                })
            }

            fn app_state<F: #Factor>(app_state: &Self::AppState) -> Option<&F::AppState> {
                #(
                    if let Some(state) = &app_state.#factor_names {
                        if let Some(state) = <dyn #Any>::downcast_ref(state) {
                            return Some(state)
                        }
                    }
                )*
                None
            }

            fn instance_builder_mut<F: #Factor>(
                builders: &mut Self::InstanceBuilders,
            ) -> Option<Option<&mut F::InstanceBuilder>> {
                let type_id = #TypeId::of::<F>();
                #(
                    if type_id == #TypeId::of::<#factor_types>() {
                        return Some(
                            builders.#factor_names.as_mut().map(|builder| {
                                <dyn #Any>::downcast_mut(builder).unwrap()
                            })
                        );
                    }
                )*
                None
            }
        }

        #vis struct #app_state_name {
            #(
                pub #factor_names: Option<<#factor_types as #Factor>::AppState>,
            )*
        }

        #vis struct #builders_name {
            #(
                pub #factor_names: Option<<#factor_types as #Factor>::InstanceBuilder>,
            )*
        }

        #vis struct #state_name {
            #(
                pub #factor_names: #factors_path::FactorInstanceState<#factor_types>,
            )*
        }

        impl #factors_path::GetFactorState for #state_name {
            fn get<F: #Factor>(&mut self) -> ::std::option::Option<&mut #factors_path::FactorInstanceState<F>> {
                #(
                    if let Some(state) = (&mut self.#factor_names as &mut (dyn #Any + #Send)).downcast_mut() {
                        return Some(state)
                    }
                )*
                None
            }
        }
    })
}
