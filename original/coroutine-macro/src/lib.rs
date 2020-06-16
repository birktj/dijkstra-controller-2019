extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ImplItemMethod};


#[proc_macro_attribute]
pub fn coroutine(attr: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as ImplItemMethod);

    let attr = if attr.is_empty() {
        quote!()
    }
    else {
        let stream = proc_macro2::TokenStream::from(attr);
        quote!(+ #stream)
    };

    let attrs = input.attrs;
    let vis = input.vis;
    let defaultness = input.defaultness;
    let constness = input.sig.constness;
    let asyncness = input.sig.asyncness;
    let unsafety  = input.sig.unsafety;
    let abi       = input.sig.abi;
    let ident     = input.sig.ident;
    let mut generics  = input.sig.decl.generics;
    let mut inputs    = input.sig.decl.inputs;
    let return_type = match input.sig.decl.output {
        syn::ReturnType::Default => quote!(()),
        syn::ReturnType::Type(_, ty) => quote!(#ty),
    };
    let block = input.block;

    let lifetime = inputs.iter_mut().filter_map(|arg| {
        match arg {
            syn::FnArg::SelfRef(ref mut lt) => {
                if lt.lifetime.is_none() {
                    let lifetime = syn::Lifetime::new("'xxxxxxselflifetime", proc_macro2::Span::call_site());
                    lt.lifetime = Some(lifetime.clone());
                    generics.params.push(syn::GenericParam::Lifetime(syn::LifetimeDef {
                        attrs: vec![],
                        lifetime,
                        colon_token: None,
                        bounds: syn::punctuated::Punctuated::new(),
                    }));
                }
                lt.lifetime.clone()
            }
            _ => None,
        }
    }).next().map(|x| quote!(+ #x));

    let where_caluse = &generics.where_clause;

    // Build the output, possibly using quasi-quotation
    let expanded = quote! {
        /*#attrs*/ #vis #defaultness #constness #asyncness #unsafety #abi fn #ident #generics (#inputs)
            -> impl core::ops::Generator<Yield = (), Return = #return_type > #lifetime #attr #where_caluse  {
                move || {
                    return #block;
                    //yield ();
                }
            }
    };

    //panic!("Expanded: {}", expanded);

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}
