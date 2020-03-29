extern crate proc_macro;

use darling::FromMeta;
use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(
    Observe,
    attributes(computed, reaction, autorun, fut_autorun, fut_reaction)
)]
pub fn observe_derive(_original: TokenStream) -> TokenStream {
    TokenStream::new()
}

#[derive(Default, FromMeta)]
struct FutureAttr {}

#[proc_macro_attribute]
pub fn store(_args: TokenStream, original: TokenStream) -> TokenStream {
    let mut original = syn::parse_macro_input!(original as syn::ItemStruct);

    original
        .attrs
        .push(syn::parse_quote!(#[derive(observe::Observe)]));

    let name = original.ident.clone();
    let mut computed = vec![];
    let mut reaction = vec![];
    let mut future = vec![];
    for f in &original.fields {
        for a in &f.attrs {
            if a.path.get_ident().map(|i| i.to_string()) == Some(String::from("computed")) {
                match &f.ident {
                    Some(ident) => computed.push(ident.clone()),
                    None => {}
                }
            }
            if a.path.get_ident().map(|i| i.to_string()) == Some(String::from("autorun")) {
                match &f.ident {
                    Some(ident) => reaction.push(ident.clone()),
                    None => {}
                }
            }
            if a.path.get_ident().map(|i| i.to_string()) == Some(String::from("fut_autorun")) {
                let args = a.tokens.clone().into();
                let attr_args = syn::parse_macro_input!(args as syn::AttributeArgs);
                let _args = FutureAttr::from_list(&attr_args);
                match &f.ident {
                    Some(ident) => future.push(ident.clone()),
                    None => {}
                }
            }
        }
    }

    let computed = computed.iter().map(|c| {
        // let get = format_ident!("get_{}", c);
        quote! {{
          let this = std::rc::Rc::downgrade(&self);
          self.#c.become_computed({
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
        }}
    });

    let reaction = reaction.iter().map(|r| {
        // let get = format_ident!("get_{}", c);
        quote! {{
          let this = std::rc::Rc::downgrade(&self);
          self.#r.become_autorun({
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
          });

          self.#r.update();
        }}
    });

    let future = future.iter().map(|r| {
        // let get = format_ident!("get_{}", c);
        quote! {{
          let this = std::rc::Rc::downgrade(&self);
          self.#r.become_fut_autorun(Box::new({
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

          self.#r.update();
        }}
    });

    let (impl_generics, ty_generics, where_clause) = original.generics.split_for_impl();

    let output = quote! {
      #original
      impl #impl_generics #name #ty_generics #where_clause {
        fn __init_observables(self: &std::rc::Rc<Self>) {
          #(#computed)*
          #(#reaction)*
          #(#future)*
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

    let mut arg_pat = Vec::new();
    for input in original.sig.inputs.iter() {
        match input {
            syn::FnArg::Typed(syn::PatType { pat, .. }) => {
                arg_pat.push(quote!(#pat));
            }
            _ => {}
        }
    }

    let block = proc_macro::TokenStream::from(quote! {
      {
        let this = Self::#temp_ident(
          #(#arg_pat),*
        );
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
