use std::fmt::Debug;

use proc_macro::TokenStream;
use quote::{format_ident, quote}; 
use syn::{self, DeriveInput, DataStruct, Data, Ident};
use bevy_erm_core::{ComponentMapper, DatabaseResource, AnyDatabaseResource};

#[proc_macro_derive(DBQueryDerive, attributes(main_key, table_name))]
pub fn query_derive(input: TokenStream) -> TokenStream {
    //TODO fix assumptions
    // key parameter is called `id`
    // id is always the first field

    // Parse the input tokens into a syntax tree
    let ast: DeriveInput = syn::parse(input).expect("Failed to parse input");

    // Extract necessary information from the input
    let ident = &ast.ident;



    let Data::Struct(data) = ast.data else {
        panic!("This derive macro only supports structs");
    };

    let table_name_meta = ast.attrs.iter().find(|attr| attr.path().is_ident("table_name")).unwrap().clone().meta;
    let syn::Meta::NameValue(name_value) = table_name_meta else {
        panic!("table_name attribute must be a name value pair");
    };

    let syn::Expr::Lit(table_name) = name_value.value else {
        panic!("table_name attribute must be a string");
    };

    let syn::Lit::Str(table_name) = table_name.lit else {
        panic!("table_name attribute must be a string");
    };


    let table_name = table_name.value();



    // Iterate through the fields of the struct
    // let main_key_field =
    //     data.fields.iter().find(
    //         |field| field.attrs.iter().find(|attr| attr.path().is_ident("main_key")).is_some()).unwrap().clone().ident.unwrap();

    let main_key_field = syn::parse_str::<Ident>("id").unwrap();


    let field_names : Vec<String> = data.fields.iter()
    .filter( |field| field.ident != Some(main_key_field.clone()))
    .map(|field| {
        let s = field.ident.clone().unwrap().to_string();
        s.into()
    }).collect();

    println!("{:?}", field_names);

    // select query
    let selection_terms = field_names.join(", ");
    let selection_query = format!("SELECT {}, {} FROM {} WHERE {} = ?", main_key_field.to_string(), selection_terms, table_name, main_key_field.to_string());
    println!("{}", selection_query);

    let update_terms = field_names.join(" = ?, ");
    let update_query = format!("UPDATE {} SET {} = ? WHERE {} = ?", table_name, update_terms, main_key_field.to_string());

    let binds = field_names.iter()
        .filter(|field| field.as_str() != "id")
        .map(|field| format_ident!("{}", field));

    let binds = quote! {
        #(.bind(component.#binds.clone()))*
    };

    println!("{}", update_query);
    println!("{}", binds);

    let insert_terms = field_names.join(", ");
    let question_marks = field_names.iter().map(|_| "?").collect::<Vec<&str>>().join(", ");
    let insert_query = format!("INSERT INTO {} ({}, {}) VALUES (?,{})", table_name, main_key_field.to_string(), insert_terms, question_marks);
    println!("{}", insert_query);

    // Generate the implementation of the IndexInfo trait
    let gen = quote! {
        use bevy_erm_core::*;

        #[async_trait]
        impl ComponentMapper for #ident {
            type Component = #ident;
            type Executor = <bevy_erm_core::AnyDatabaseResource as DatabaseResource>::Transaction;
        
            async fn get<'c>(
                e: &mut Self::Executor,
                db_entity: &DatabaseEntityId,
            ) -> Result<Self::Component, ()> {
                let mut guard = e.lock().await;
                let tr = guard.a.as_mut().unwrap();

                let items = sqlx::query_as::<_, #ident>(#selection_query)
                    .bind(db_entity)
                    .fetch_one(&mut **tr)
                    .await
                    .unwrap();

                Ok(items)
            }
        
            async fn update_component<'c>(
                tr: &mut Self::Executor,
                db_entity: &DatabaseEntityId,
                component: &Self::Component,
            ) -> Result<(), ()> {
                let mut guard = tr.lock().await;
                let tr = guard.a.as_mut().unwrap();

                let r = sqlx::query(#update_query)
                    #binds
                    .bind(db_entity)
                    .execute(&mut **tr)
                    .await;
                
                match r {
                    Ok(_) => Ok(()),
                    Err(_) => Err(()),
                }
            }
        
            async fn insert_component<'c>(
                tr: &mut Self::Executor,
                db_entity: &DatabaseEntityId,
                component: &Self::Component,
            ) -> Result<(), ()> {
                let mut guard = tr.lock().await;
                let tr = guard.a.as_mut().unwrap();

                let r = sqlx::query(#insert_query)
                    .bind(db_entity)
                    #binds
                    .execute(&mut **tr)
                    .await;

                match r {
                    Ok(_) => Ok(()),
                    Err(_) => Err(()),
                }
            }
        }
    };

    // Convert the generated code into a token stream and return it
    gen.into()
}