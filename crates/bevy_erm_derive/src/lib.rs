use casey::lower;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DataStruct, DeriveInput, Ident};
extern crate proc_macro;
extern crate casey;
extern crate quote;

#[proc_macro_derive(DBQueryDerive, attributes(main_key, table_name))]
pub fn query_derive(input: TokenStream) -> TokenStream {
    //TODO fix assumptions
    // key parameter is called `id`
    // id is always the first field

    // Parse the input tokens into a syntax tree
    let ast: DeriveInput = syn::parse(input).expect("Failed to parse input");

    let Data::Struct(ref data) = ast.data else {
        panic!("This derive macro only supports structs");
    };

    if data.fields.is_empty() {
        marker_component(&ast, data)
    } else {
        full_component(&ast, data)
    }
}

fn get_load_all_query_impl(
    ast: &DeriveInput,
    _data: &DataStruct,
    load_all_query: String,
) -> proc_macro2::TokenStream {
    let ident = &ast.ident;

    let load_all_struct = format_ident!("{}QueryLoadAll", ident);

    quote!(
        pub struct #load_all_struct(pub RequestId);

        #[async_trait]
        impl CustomDatabaseQuery<SqlxSqliteDatabaseResource, #ident> for #load_all_struct {
            async fn query(
                &self,
                tr: DatabaseTransaction<SqlxSqliteDatabaseResource>,
            ) -> Result<Vec<(DatabaseEntity, #ident)>, ()> {
                let mut guard = tr.lock().await;
                let tr = guard.a.as_mut().unwrap();
                let db_entity_and_components = sqlx::query_as::<_, DataseBaseEntityAndComponent<#ident>>(#load_all_query)
                    .fetch_all(&mut **tr)
                    .await
                    .unwrap();

                let db_entity_and_components = db_entity_and_components
                    .into_iter()
                    .map(|db_entity_and_component| {
                        let mut entity = db_entity_and_component.entity;
                        entity.request = self.0;
                        (
                            entity,
                            db_entity_and_component.component,
                        )
                    })
                    .collect();

                Ok(db_entity_and_components)
            }
        }
    )
}

fn marker_component(ast: &DeriveInput, data: &DataStruct) -> TokenStream {
    let ident = &ast.ident;

    let marker_col = lower!(ident);

    let table_name = get_table_name(ast);
    let main_key_field = get_main_key(ast);

    let selection_query = format!(
        "SELECT {} FROM {} WHERE {} = ?",
        marker_col, table_name, main_key_field
    );

    let update_query = format!(
        "UPDATE {} SET {} = ? WHERE {} = ?",
        table_name, marker_col, main_key_field
    );

    let load_all_query = format!(
        "SELECT {} FROM {} WHERE {} = ?",
        main_key_field, table_name, marker_col
    );

    let load_all_query_impl = get_load_all_query_impl(ast, data, load_all_query);

    let gen = quote! {
        use bevy_erm_core::*;

        #[async_trait]
        impl ComponentMapper for #ident {
            type Component = #ident;
            type Executor = <bevy_erm_core::SqlxSqliteDatabaseResource as DatabaseResource>::Transaction;

            async fn get<'c>(
                e: &mut Self::Executor,
                db_entity: &DatabaseEntityId,
            ) -> Result<Self::Component, ()> {
                let mut guard = e.lock().await;
                let tr = guard.a.as_mut().unwrap();

                let marker_bool = sqlx::query(#selection_query)
                    .bind(db_entity)
                    .fetch_one(&mut **tr)
                    .await;

                match marker_bool {
                    Ok(_) => Ok(#ident {}),
                    Err(_) => Err(()),
                }
            }

            async fn update_component<'c>(
                tr: &mut Self::Executor,
                db_entity: &DatabaseEntityId,
                component: &Self::Component,
            ) -> Result<(), ()> {
                // Can't really imaging that this is ever called for a marker component
                Ok(())
            }

            async fn insert_component<'c>(
                tr: &mut Self::Executor,
                db_entity: &DatabaseEntityId,
                component: &Self::Component,
            ) -> Result<(), ()> {
                let mut guard = tr.lock().await;
                let tr = guard.a.as_mut().unwrap();

                let r = sqlx::query(#update_query)
                    .bind(true)
                    .bind(db_entity)
                    .execute(&mut **tr)
                    .await;

                match r {
                    Ok(_) => Ok(()),
                    Err(_) => Err(()),
                }
            }
        }

        #load_all_query_impl


    };

    gen.into()
}

fn get_table_name(ast: &DeriveInput) -> String {
    let table_name_meta = ast
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("table_name"))
        .expect("No table name provided")
        .clone()
        .meta;
    let syn::Meta::NameValue(name_value) = table_name_meta else {
        panic!("table_name attribute must be a name value pair");
    };

    let syn::Expr::Lit(table_name) = name_value.value else {
        panic!("table_name attribute must be a string");
    };

    let syn::Lit::Str(table_name) = table_name.lit else {
        panic!("table_name attribute must be a string");
    };

    table_name.value()
}

fn get_main_key(_ast: &DeriveInput) -> Ident {
    syn::parse_str::<Ident>("id").unwrap()
}

fn full_component(ast: &DeriveInput, data: &DataStruct) -> TokenStream {
    // Extract necessary information from the input
    let ident = &ast.ident;

    let table_name = get_table_name(ast);

    // Iterate through the fields of the struct
    // let main_key_field =
    //     data.fields.iter().find(
    //         |field| field.attrs.iter().find(|attr| attr.path().is_ident("main_key")).is_some()).unwrap().clone().ident.unwrap();

    let main_key_field = get_main_key(ast);

    let field_names: Vec<String> = data
        .fields
        .iter()
        .filter(|field| field.ident != Some(main_key_field.clone()))
        .map(|field| field.ident.clone().unwrap().to_string())
        .collect();


    // select query
    let selection_terms = field_names.join(", ");
    let selection_query = format!(
        "SELECT {}, {} FROM {} WHERE {} = ?",
        main_key_field, selection_terms, table_name, main_key_field
    );

    let update_terms = field_names.join(" = ?, ");
    let update_query = format!(
        "UPDATE {} SET {} = ? WHERE {} = ?",
        table_name, update_terms, main_key_field
    );

    let binds = field_names
        .iter()
        .filter(|field| main_key_field != field.as_str())
        .map(|field| format_ident!("{}", field));

    let binds = quote! {
        #(.bind(component.#binds.clone()))*
    };

    let load_all_query = format!(
        "SELECT {}, {} FROM {}",
        main_key_field, selection_terms, table_name
    );

    let load_all_query_impl = get_load_all_query_impl(ast, data, load_all_query);


    let insert_terms = field_names.join(", ");
    let question_marks = field_names
        .iter()
        .map(|_| "?")
        .collect::<Vec<&str>>()
        .join(", ");
    let insert_query = format!(
        "INSERT INTO {} ({}, {}) VALUES (?,{})",
        table_name, main_key_field, insert_terms, question_marks
    );

    // Generate the implementation of the IndexInfo trait
    let gen = quote! {
        use bevy_erm_core::*;

        #[async_trait]
        impl ComponentMapper for #ident {
            type Component = #ident;
            type Executor = <bevy_erm_core::SqlxSqliteDatabaseResource as DatabaseResource>::Transaction;

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

        #load_all_query_impl

    };

    // Convert the generated code into a token stream and return it
    gen.into()
}
