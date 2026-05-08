use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{Data, DeriveInput, Fields, Ident, Type, parse_macro_input};
use quote::quote;

fn is_optional<'a>(ty: &'a Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            let path = &type_path.path;
            let segments = &path.segments;

            if segments.is_empty() {
                return false;
            }

            let last_segment = &segments[segments.len() - 1];
            let ident = &last_segment.ident;

            return ident == "Option";
        },
        _ => return false
    }
}

fn parse_attr_fields(attr: &syn::Attribute) -> Result<Vec<syn::Ident>, syn::Error> {
    let mut fields = Vec::new();
    attr.parse_nested_meta(|meta| {
        if let Some(ident) = meta.path.get_ident() {
            fields.push(ident.clone());
            Ok(())
        } else {
            Err(meta.error("expected identifier"))
        }
    })?;
    return Ok(fields);
}

#[derive(Debug)]
enum Constraint {
    Unique(Vec<Ident>),
}

impl Constraint {
    fn to_sql(&self) -> String {
        match self {
            Constraint::Unique(idents) => format!("unique ({})", idents.iter().map(|ident| ident.to_string()).collect::<Vec<_>>().join(",")),
        }
    }
}

fn parse_attrs(attrs: Vec<syn::Attribute>) -> Result<Vec<Constraint>, syn::Error> {
    let mut constraints = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("unique") {
            let fields = parse_attr_fields(&attr)?;
            constraints.push(Constraint::Unique(fields));
        }
    }

    Ok(constraints)
}

#[proc_macro_derive(BaseModel, attributes(unique))]
pub fn derive_basemodel(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let constraints = parse_attrs(input.attrs).unwrap();
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => panic!("Derive macro for `modelite::BaseModel` only supports structs"),
    };

    TokenStream::from(basemodel_tokens(name, fields, constraints))
}

// TODO: #[unique], #[primary_key], #[foreign_key(Other::id)]
fn basemodel_tokens(name: &Ident, fields: &Fields, constraints: Vec<Constraint>) -> TokenStream2 {
    let name_str = name.to_string();

    let fields_parsed = fields.iter().map(|field| (field.ident.as_ref().unwrap(), &field.ty, is_optional(&field.ty)));
    let field_names = fields_parsed.clone().map(|field| field.0.to_string());
    let field_decls = fields_parsed.map(|field| {
        let field_name = field.0.to_string();
        let field_type = field.1;
        let null = if field.2 { "NULL" } else { "NOT NULL" };

        quote!(
            format!("\"{}\" {} {}", #field_name, ::sqlx::TypeInfo::name(&<#field_type as ::sqlx::Type<::sqlx::Sqlite>>::type_info()), #null)
        )
    });

    let constraints = if constraints.len() == 0 {
        String::from("")
    } else {
        String::from(",") + &constraints.into_iter().map(|constraint| constraint.to_sql()).collect::<Vec<_>>().join(",")
    };

    let stream = quote! {
        impl ::modelite::BaseModel for #name {
            const COLUMNS: &'static [&'static str] = &[#(#field_names,)*];

            fn table_name() -> &'static str {
                #name_str
            }

            fn create_table_sql() -> &'static str {
                static CACHE: ::std::sync::OnceLock<String> = ::std::sync::OnceLock::new();
                CACHE.get_or_init(|| {
                    ::modelite::util::create_table_sql::<Self>(format!("{}{}", [#(#field_decls,)*].join(", "), #constraints).into())
                })
            }

            fn insert_sql() -> &'static str {
                static CACHE: ::std::sync::OnceLock<String> = ::std::sync::OnceLock::new();
                CACHE.get_or_init(|| {
                    ::modelite::util::insert_sql::<Self>()
                })
            }

            fn insert_one_sql() -> &'static str {
                static CACHE: ::std::sync::OnceLock<String> = ::std::sync::OnceLock::new();
                CACHE.get_or_init(|| {
                    ::modelite::util::insert_one_sql::<Self>()
                })
            }

            fn select_sql() -> &'static str {
                static CACHE: ::std::sync::OnceLock<String> = ::std::sync::OnceLock::new();
                CACHE.get_or_init(|| {
                    ::modelite::util::select_sql::<Self>()
                })
            }

            fn drop_table_sql() -> &'static str {
                static CACHE: ::std::sync::OnceLock<String> = ::std::sync::OnceLock::new();
                CACHE.get_or_init(|| {
                    ::modelite::util::drop_table_sql::<Self>()
                })
            }
        }
    };

    stream
}

#[cfg(feature = "sqlx")]
#[proc_macro_derive(Model, attributes(unique))]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let constraints = parse_attrs(input.attrs).unwrap();
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => panic!("Derive macro for `modelite::Model` only supports structs"),
    };

    let tokens1 = basemodel_tokens(name, fields, constraints);
    let tokens2 = model_tokens(name, fields);

    let tokens = TokenStream::from(quote! {
        #tokens1
        #tokens2
    });

    return tokens;
}

#[cfg(feature = "sqlx")]
fn model_tokens(name: &Ident, fields: &Fields) -> TokenStream2 {
    let push_binds = fields.iter().map(|field| {
        let field = field.ident.as_ref().unwrap();
        quote! {
            .push_bind(&d.#field)
        }
    });

    let binds = fields.iter().map(|field| {
        let field = field.ident.as_ref().unwrap();
        quote! {
            .bind(&value.#field)
        }
    });

    quote! {
        impl ::modelite::Model for #name {
            fn insert<'s>(value: &'s Self) -> ::sqlx::query::Query<'s, ::sqlx::Sqlite, < ::sqlx::Sqlite as ::sqlx::Database>::Arguments<'s>> {
                let query = ::sqlx::query(<Self as ::modelite::BaseModel>::insert_one_sql());
                query #(#binds)*
            }

            fn insert_all_unchecked<'s>(values: impl ::core::iter::IntoIterator<Item = &'s Self>) -> ::sqlx::QueryBuilder<'s, ::sqlx::Sqlite> {
                let mut qb = ::sqlx::QueryBuilder::new(<Self as ::modelite::BaseModel>::insert_sql());

                qb.push_values(values, |mut b, d| {
                    b #(#push_binds)*;
                });

                return qb;
            }
        }
    }
}
