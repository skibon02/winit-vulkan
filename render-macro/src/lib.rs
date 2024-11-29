mod define_layout;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

#[proc_macro_derive(CollectDrawStateUpdates)]
pub fn derive_collect_draw_state_updates(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let (uniform_buffer_updates, uniform_image_updates, object_updates, clear_updates) =
        if let Data::Struct(data) = &input.data {
            match &data.fields {
                Fields::Named(fields) => {
                    let uniform_buffer_calls: Vec<_> = fields.named.iter().map(|f| {
                        let field_name = &f.ident;
                        quote! {
                            self.#field_name.collect_uniform_buffer_updates()
                        }
                    }).collect();
                    let first_uniform_buffer_call = uniform_buffer_calls.first();
                    let rest_uniform_buffer_calls = uniform_buffer_calls.iter().skip(1);

                    let uniform_image_calls: Vec<_> = fields.named.iter().map(|f| {
                        let field_name = &f.ident;
                        quote! {
                            self.#field_name.collect_uniform_image_updates()
                        }
                    }).collect();
                    let first_uniform_image_call = uniform_image_calls.first();
                    let rest_uniform_image_calls = uniform_image_calls.iter().skip(1);

                    let object_calls: Vec<_> = fields.named.iter().map(|f| {
                        let field_name = &f.ident;
                        quote! {
                            self.#field_name.collect_object_updates()
                        }
                    }).collect();
                    let first_object_call = object_calls.first();
                    let rest_object_calls = object_calls.iter().skip(1);

                    let clear_calls: Vec<_> = fields.named.iter().map(|f| {
                        let field_name = &f.ident;
                        quote! {
                            self.#field_name.clear_updates();
                        }
                    }).collect();

                    (
                        quote! {
                            #first_uniform_buffer_call
                            #(.chain(#rest_uniform_buffer_calls))*
                        },
                        quote! {
                            #first_uniform_image_call
                            #(.chain(#rest_uniform_image_calls))*
                        },
                        quote! {
                            #first_object_call
                            #(.chain(#rest_object_calls))*
                        },
                        quote! {
                            #(#clear_calls)*
                        },
                    )
                }
                Fields::Unnamed(fields) => {
                    let mut uniform_buffer_calls = fields.unnamed.iter().enumerate().map(|(i, _)| {
                        let index = syn::Index::from(i);
                        quote! {
                            self.#index.collect_uniform_buffer_updates()
                        }
                    });
                    let first_uniform_buffer_call = uniform_buffer_calls.next();

                    let mut uniform_image_calls = fields.unnamed.iter().enumerate().map(|(i, _)| {
                        let index = syn::Index::from(i);
                        quote! {
                            self.#index.collect_uniform_image_updates()
                        }
                    });
                    let first_uniform_image_call = uniform_image_calls.next();

                    let mut object_calls = fields.unnamed.iter().enumerate().map(|(i, _)| {
                        let index = syn::Index::from(i);
                        quote! {
                            self.#index.collect_object_updates()
                        }
                    });
                    let first_object_call = object_calls.next();

                    let clear_calls = fields.unnamed.iter().enumerate().map(|(i, _)| {
                        let index = syn::Index::from(i);
                        quote! {
                            self.#index.clear_updates();
                        }
                    });

                    (
                        quote! {
                            #first_uniform_buffer_call
                            #(.chain(#uniform_buffer_calls))*
                        },
                        quote! {
                            #first_uniform_image_call
                            #(.chain(#uniform_image_calls))*
                        },
                        quote! {
                            #first_object_call
                            #(.chain(#object_calls))*
                        },
                        quote! {
                            #(#clear_calls)*
                        },
                    )
                }
                Fields::Unit => (
                    quote! { iter::empty() },
                    quote! { iter::empty() },
                    quote! { iter::empty() },
                    quote! {},
                ),
            }
        } else {
            (
                quote! { iter::empty() },
                quote! { iter::empty() },
                quote! { iter::empty() },
                quote! {},
            )
        };

    let expanded = quote! {
        impl render_core::collect_state::CollectDrawStateUpdates for #name {
            fn collect_uniform_buffer_updates(&self) -> impl Iterator<Item=(<render_core::UniformBufferUpdatesDesc as render_core::UpdatesDesc>::ID, render_core::StateUpdates<render_core::UniformBufferUpdatesDesc>)> {
                #uniform_buffer_updates
            }

            fn collect_uniform_image_updates(&self) -> impl Iterator<Item=(<render_core::UniformImageUpdatesDesc as render_core::UpdatesDesc>::ID, render_core::StateUpdates<render_core::UniformImageUpdatesDesc>)> {
                #uniform_image_updates
            }

            fn collect_object_updates(&self) -> impl Iterator<Item=(<render_core::ObjectUpdatesDesc as render_core::UpdatesDesc>::ID, render_core::StateUpdates<render_core::ObjectUpdatesDesc>)> {
                #object_updates
            }

            fn clear_updates(&mut self) {
                #clear_updates
            }
        }
    };

    TokenStream::from(expanded)
}



#[proc_macro]
pub fn define_layout(input: TokenStream) -> TokenStream {
    define_layout::define_layout(input)
}