extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};

#[proc_macro_derive(Observe, attributes(computed, value, reaction))]
pub fn observe_derive(original: TokenStream) -> TokenStream {
    // let original = syn::parse_macro_input!(original as syn::ItemStruct);
    TokenStream::new()
}

#[proc_macro_attribute]
pub fn store(_args: TokenStream, original: TokenStream) -> TokenStream {
    let mut original = syn::parse_macro_input!(original as syn::ItemStruct);
    // let derive = "#[derive(observe::Observe)]".parse().unwrap();
    // let parser = syn::parse_str("#[derive(observe::Observe)]").unwrap();
    // let derive = syn::Attribute::parse_outer(parser);

    original
        .attrs
        .push(syn::parse_quote!(#[derive(observe::Observe)]));

    let name = original.ident.clone();
    let mut computed = vec![];
    let mut reaction = vec![];
    for f in &original.fields {
        match &f.ty {
            syn::Type::Path(path) => {
                for seg in &path.path.segments {
                    if seg.ident.to_string() == "Computed" {
                        match &f.ident {
                            Some(ident) => computed.push(ident.clone()),
                            None => {}
                        }
                    } else if seg.ident.to_string() == "Reaction" {
                        match &f.ident {
                            Some(ident) => reaction.push(ident.clone()),
                            None => {}
                        }
                    }
                }
            }
            _ => {}
        };
    }

    let computed = computed.iter().map(|c| {
        // let get = format_ident!("get_{}", c);
        quote! {
          self.#c.set_handler({
            let this = Arc::downgrade(&self);
            move |ctx| {
              let arc = this.upgrade();
              match arc {
                Some(store) => {
                  store.#c(ctx)
                },
                None => {
                  panic!("Call on a dropped store")
                }
              }
            }
          });
        }
    });

    let reaction = reaction.iter().map(|r| {
        // let get = format_ident!("get_{}", c);
        quote! {
          self.#r.set_handler(observe::Autorun::new({
            let this = Arc::downgrade(&self);
            move |ctx| {
              let arc = this.upgrade();
              match arc {
                Some(store) => {
                  store.#r(ctx)
                },
                None => {
                  panic!("Call on a dropped store")
                }
              }
            }
          }));

          self.#r.run();
        }
    });

    let output = quote! {
      #original
      impl #name {
        fn __init_observables(self: &Arc<Self>) {
          #(#computed)*
          #(#reaction)*
        }
      }
    };

    proc_macro::TokenStream::from(output)
}

#[proc_macro_attribute]
pub fn create(_args: TokenStream, original: TokenStream) -> TokenStream {
    let mut original = syn::parse_macro_input!(original as syn::ImplItemMethod);
    let temp_ident = syn::Ident::new(
        &format!("__private_{}", original.sig.ident),
        original.sig.ident.span(),
    );

    let mut new_fn = original.clone();
    let block = proc_macro::TokenStream::from(quote! {
      {
        let this = Self::#temp_ident();
        this.__init_observables();
        this
      }
    });

    new_fn.block = syn::parse_macro_input!(block as syn::Block);

    original.sig.ident = temp_ident;
    original.vis = syn::Visibility::Inherited;

    let output = quote! {
      #original
      #new_fn
    };

    proc_macro::TokenStream::from(output)
}

#[proc_macro_attribute]
pub fn value(_args: TokenStream, original: TokenStream) -> TokenStream {
    // let original = syn::parse_macro_input!(original as syn::ItemStruct);
    original
}

#[proc_macro_attribute]
pub fn computed(_args: TokenStream, original: TokenStream) -> TokenStream {
    // let original = syn::parse_macro_input!(original as syn::ItemStruct);
    original
}

#[proc_macro_attribute]
pub fn action(_args: TokenStream, original: TokenStream) -> TokenStream {
    // let original = syn::parse_macro_input!(original as syn::ItemStruct);
    original
}
