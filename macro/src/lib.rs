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

#[proc_macro_derive(BaseModel)]
pub fn derive_basemodel(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => panic!("Derive macro for `modelite::BaseModel` only supports structs"),
    };

    TokenStream::from(basemodel_tokens(name, fields))
}

fn basemodel_tokens(name: &Ident, fields: &Fields) -> TokenStream2 {
    let name_str = name.to_string();

    let fields_len = fields.len();
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

    let stream = quote! {
        impl ::modelite::BaseModel<#fields_len> for #name {
            const COLUMNS: [&'static str; #fields_len] = [#(#field_names,)*];

            fn table_name() -> String {
                #name_str.to_string()
            }

            fn create_table_sql() -> String {
                format!("CREATE TABLE IF NOT EXISTS \"{}\" ({})",
                    Self::table_name(),
                    [#(#field_decls,)*].join(", ")
                )
            }
        }
    };

    stream
}

#[cfg(feature = "sqlx")]
#[proc_macro_derive(Model)]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => panic!("Derive macro for `modelite::Model` only supports structs"),
    };

    let tokens1 = basemodel_tokens(name, fields);
    let tokens2 = model_tokens(name, fields);

    TokenStream::from(quote! {
        #tokens1
        #tokens2
    })
}

#[cfg(feature = "sqlx")]
fn model_tokens(name: &Ident, fields: &Fields) -> TokenStream2 {
    let fields_len = fields.len();

    let push_binds = fields.iter().map(|field| {
        let field = field.ident.as_ref().unwrap();
        quote! {
            .push_bind(&d.#field)
        }
    });

    quote! {
        impl ::modelite::Model<#fields_len> for #name {
            async fn insert_bulk<'e, 'a, E>(e: E, values: impl ::core::iter::IntoIterator<Item = &'a Self>) -> ::core::result::Result<::sqlx::sqlite::SqliteQueryResult, ::sqlx::Error>
                where
                    Self: 'a,
                    E: ::sqlx::Executor<'a, Database = ::sqlx::Sqlite>,
            {
                let mut qb = ::sqlx::QueryBuilder::new(Self::insert_sql());

                qb.push_values(values, |mut b, d| {
                    b #(#push_binds)*;
                }).build().execute(e).await
            }
        }
    }
}
