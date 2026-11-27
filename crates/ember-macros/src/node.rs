//! Implementation of `#[derive(Node)]`.

use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::{Data, DeriveInput, Ident, LitStr, Result, Type};

pub fn derive(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let class_name = parse_class_name(&input)?;
    verify_struct_has_name_field(&input)?;
    let properties = property_methods(&exported_fields(&input)?);

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
            #properties
        }
    })
}

/// An exported field: its name, type and `#[export(enum = "…")]` values
/// (ADR-049).
struct Exported {
    ident: Ident,
    ty: Type,
    values: Option<Vec<String>>,
}

/// `#[export]` fields, in declaration order (ADR-037).
fn exported_fields(input: &DeriveInput) -> Result<Vec<Exported>> {
    let Data::Struct(data) = &input.data else {
        return Ok(Vec::new()); // reported by verify_struct_has_name_field
    };
    let mut out = Vec::new();
    for field in &data.fields {
        let mut export = false;
        let mut values = None;
        for attr in field.attrs.iter().filter(|a| a.path().is_ident("export")) {
            match &attr.meta {
                syn::Meta::Path(_) => export = true,
                syn::Meta::List(_) => {
                    attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("skip") {
                            Ok(())
                        } else if meta.path.is_ident("enum") {
                            let list: LitStr = meta.value()?.parse()?;
                            let names: Vec<String> = list
                                .value()
                                .split(',')
                                .map(|n| n.trim().to_string())
                                .collect();
                            if names.iter().any(String::is_empty) {
                                return Err(syn::Error::new_spanned(
                                    &list,
                                    "#[export(enum = \"A,B\")] needs non-empty names",
                                ));
                            }
                            export = true;
                            values = Some(names);
                            Ok(())
                        } else {
                            Err(meta.error(
                                "expected #[export], #[export(skip)] or #[export(enum = \"A,B\")]",
                            ))
                        }
                    })?;
                }
                syn::Meta::NameValue(nv) => {
                    return Err(syn::Error::new_spanned(nv, "expected #[export]"));
                }
            }
        }
        if !export {
            continue;
        }
        let Some(ident) = field.ident.clone() else {
            return Err(syn::Error::new_spanned(
                field,
                "#[export] needs a named field",
            ));
        };
        if ident == "name" {
            return Err(syn::Error::new_spanned(
                field,
                "`name` is the node's name and is always saved; remove #[export]",
            ));
        }
        out.push(Exported {
            ident,
            ty: field.ty.clone(),
            values,
        });
    }
    Ok(out)
}

/// `property_list` / `get_property` / `set_property` for the exported
/// fields. Nothing when there are none: the `NodeName` defaults apply.
fn property_methods(fields: &[Exported]) -> TokenStream {
    if fields.is_empty() {
        return TokenStream::new();
    }
    let keys: Vec<String> = fields.iter().map(|f| f.ident.unraw().to_string()).collect();
    let idents: Vec<&Ident> = fields.iter().map(|f| &f.ident).collect();
    let types: Vec<&Type> = fields.iter().map(|f| &f.ty).collect();
    let hinted = fields.iter().filter_map(|f| {
        let values = f.values.as_ref()?;
        let key = f.ident.unraw().to_string();
        Some(quote! { #key => ::ember_scene::PropertyHint::Enum(&[#(#values),*]), })
    });
    let hint = quote! {
        fn property_hint(&self, key: &str) -> ::ember_scene::PropertyHint {
            match key {
                #(#hinted)*
                _ => ::ember_scene::PropertyHint::None,
            }
        }
    };
    quote! {
        #hint
        fn property_list(&self) -> &'static [&'static str] {
            &[#(#keys),*]
        }
        fn get_property(&self, key: &str) -> ::std::option::Option<::ember_core::Variant> {
            match key {
                #(#keys => ::std::option::Option::Some(
                    ::ember_core::VariantValue::to_variant(&self.#idents)
                ),)*
                _ => ::std::option::Option::None,
            }
        }
        fn set_property(&mut self, key: &str, value: &::ember_core::Variant) -> bool {
            match key {
                #(#keys => match <#types as ::ember_core::VariantValue>::from_variant(value) {
                    ::std::option::Option::Some(v) => {
                        self.#idents = v;
                        true
                    }
                    ::std::option::Option::None => false,
                },)*
                _ => false,
            }
        }
    }
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
