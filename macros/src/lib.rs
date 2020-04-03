extern crate proc_macro;

use darling::FromMeta;
use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(Observe, attributes(computed, reaction, autorun))]
pub fn observe_derive(_original: TokenStream) -> TokenStream {
    TokenStream::new()
}

#[derive(Default, FromMeta)]
struct StoreAttr {
    #[darling(default)]
    shared: bool,
}

#[derive(Default, FromMeta)]
struct AutorunAttr {
    #[darling(default)]
    future: Option<Runtime>,
}

#[derive(FromMeta, PartialEq, Eq, Debug)]
enum Runtime {
    Tokio,
    Wasm,
}

#[proc_macro_attribute]
pub fn store(args: TokenStream, original: TokenStream) -> TokenStream {
    let mut original = syn::parse_macro_input!(original as syn::ItemStruct);

    let store_args =
        StoreAttr::from_list(&syn::parse_macro_input!(args as syn::AttributeArgs)).unwrap();

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
                let args = a.parse_args::<proc_macro2::TokenStream>();
                let args = if args.is_ok() {
                    let args = args.unwrap().into();
                    let args = syn::parse_macro_input!(args as syn::AttributeArgs);
                    AutorunAttr::from_list(&args).unwrap()
                } else {
                    AutorunAttr::default()
                };

                match &f.ident {
                    Some(ident) => {
                        if args.future.is_some() {
                            future.push((ident.clone(), args.future.unwrap()))
                        } else {
                            reaction.push(ident.clone())
                        }
                    }
                    None => {}
                }
            }
        }
    }

    let pointer = if store_args.shared {
        quote! {std::sync::Arc}
    } else {
        quote! {std::rc::Rc}
    };

    let computed = computed.iter().map(|c| {
        // let get = format_ident!("get_{}", c);
        quote! {{
          let this = #pointer::downgrade(&self);
          self.#c.set_computation(Box::new({
            observe::Computed::new(move |ctx| {
              let arc = this.upgrade();
              match arc {
                Some(store) => {
                  store.#c(ctx)
                },
                None => {
                  panic!("Call on a dropped store")
                }
              }
            })
          }));
        }}
    });

    let reaction = reaction.iter().map(|r| {
        // let get = format_ident!("get_{}", c);
        quote! {{
          let this = #pointer::downgrade(&self);
          self.#r.set_computation(Box::new({
            observe::Computed::new(move |ctx| {
              let arc = this.upgrade();
              match arc {
                Some(store) => {
                  store.#r(ctx)
                },
                None => {
                  panic!("Call on a dropped store")
                }
              }
            })
          }));

          self.#r.autorun();
          self.#r.update();
        }}
    });

    let future = future.iter().map(|(r, runtime)| {
        // let get = format_ident!("get_{}", c);
        let runtime = match runtime {
            Runtime::Tokio => quote!{observe::future::TokioRuntime},
            Runtime::Wasm => quote!{observe::future::WasmRuntime},
        };
        quote! {{
          let this = #pointer::downgrade(&self);
          let tracker = self.#r.tracker().unwrap().weak();
          let mut future = observe::future::ComputedFuture::<_,#runtime, _, _>::new(move |ctx: &mut observe::EvalContext<_>| {
            let arc = this.upgrade();
            match arc {
              Some(store) => {
                store.#r(ctx)
              },
              None => {
                panic!("Call on a dropped store")
              }
            }
          });
          future.set_tracker(tracker);
          self.#r.set_computation(Box::new(future));
        }}
    });

    let (impl_generics, ty_generics, where_clause) = original.generics.split_for_impl();

    let output = quote! {
      #original
      impl #impl_generics #name #ty_generics #where_clause {
        fn __init_observables(self: &#pointer<Self>) {
          #(#computed)*
          #(#reaction)*
          #(#future)*
        }
      }
    };

    proc_macro::TokenStream::from(output)
}

#[proc_macro_attribute]
pub fn constructor(_args: TokenStream, original: TokenStream) -> TokenStream {
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
