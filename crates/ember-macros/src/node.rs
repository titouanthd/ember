//! Implementation of `#[derive(Node)]`.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, LitStr, Result};

pub fn derive(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let class_name = parse_class_name(&input)?;
    verify_struct_has_name_field(&input)?;

    Ok(quote! {
        impl #impl_generics ::ember_core::Object for #name #ty_generics #where_clause {
            fn class_name() -> &'static str
            where
                Self: Sized,
            {
                #class_name
            }
            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn ::std::any::Any {
                self
            }
        }

        impl #impl_generics ::ember_scene::NodeName for #name #ty_generics #where_clause {
            fn name(&self) -> &str {
                &self.name
            }
            fn set_name(&mut self, name: ::std::string::String) {
                self.name = name;
            }
        }
    })
}

fn parse_class_name(input: &DeriveInput) -> Result<LitStr> {
    for attr in &input.attrs {
        if !attr.path().is_ident("class_name") {
            continue;
        }
        return match &attr.meta {
            syn::Meta::NameValue(nv) => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    Ok(s.clone())
                } else {
                    Err(syn::Error::new_spanned(
                        &nv.value,
                        "#[class_name] expects a string literal",
                    ))
                }
            }
            syn::Meta::List(list) => syn::parse2::<LitStr>(list.tokens.clone()),
            _ => Err(syn::Error::new_spanned(
                attr,
                "expected #[class_name = \"...\"]",
            )),
        };
    }
    Err(syn::Error::new_spanned(
        input,
        "#[derive(Node)] requires the attribute #[class_name = \"...\"]",
    ))
}

fn verify_struct_has_name_field(input: &DeriveInput) -> Result<()> {
    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "#[derive(Node)] can only be applied to structs",
            ));
        }
    };

    let has_name = fields.iter().any(|f| {
        f.ident.as_ref().is_some_and(|i| i == "name")
            && matches!(&f.ty, syn::Type::Path(p) if p.path.is_ident("String"))
    });

    if !has_name {
        return Err(syn::Error::new_spanned(
            input,
            "#[derive(Node)] requires a field `name: String`",
        ));
    }

    Ok(())
}
