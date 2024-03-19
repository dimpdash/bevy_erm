# Entity Relational Mapper (built on bevy)

A bevy plugin designed to aid database access using an Entity Component System architecture.

Individual user requests are passed into the ECS as events. The systems can use a database query that allows loading of entities in the ECS from a database. Once entities have been modified in memory they are flushed back to the database upong a flush event for that request. Each request opens a new database transaction maintaining isolation between requests. 

```

   ┌────────────────────────────────────────────────────┐
   │                                                    │
   │                     Web Server                     │
   │                                                    │
   │                                                    │
   │    ┌─────────────────┐      ┌─────────────────┐    │
   │    │   Requests      │      │   Responses     │    │
   │    │                 │      │                 │    │
   │    │  ┌───────┐      │      │  ┌───────┐      │    │
   │    │  │       │      │      │  │       │      │    │
   │    │  └───────┘      │      │  └───────┘      │    │
   │    │                 │      │                 │    │
   │    │  ┌───────┐      │      │  ┌───────┐      │    │
   │    │  │       │      │      │  │       │      │    │
   │    │  └───────┘      │      │  └───────┘      │    │
   │    │                 │      │                 │    │
   │    │  ┌───────┐      │      │  ┌───────┐      │    │
   │    │  │       │      │      │  │       │      │    │
   │    │  └───────┘      │      │  └───────┘      │    │
   │    │                 │      │                 │    │
   │    └───┬─────────────┘      └─────────────▲───┘    │
   │        │                                  │        │
   │        │                                  │        │
   └────────┼──────────────────────────────────┼────────┘
            │                                  │
    Requests│                                  │Events
    create  │            ┌────────┐            │create
    events  │            │        │            │response
            │          ┌─┴──────┐ │            │
            │          │        │ │            │
            └──────────► Event  │ ├────────────┘
                       │        │ │
                       │        ├─┘
                       └───┬──▲─┘
                Systems    │  │
                process    │  │
                events     │  │
                       ┌───▼──┴──────────┐
                       │                 │
                   ┌───┴───────────────┐ │
                   │   Business Logic  │ │
                   │      Systems      │ │
                   │                   │ │
                   │                   │ │
                   │                   │ │
                   │                   │ │
                   │                   │ │
                   │                   │ │
                   │                   │ │
                   │                   │ │
                   │                   │ │
                   │                   ├─┘
                   └───┬─────────────▲─┘
                       │             │
             Queries   │             │ Queries
             access the│             │ return
             database  │             │ components
         ┌─────────────▼┐           ┌┴─────────────────────────────────────┐
         │   Database   │ loaded    │  Entities                            │
         │              │ in as     │                                      │
         │              │ components│            ┌────────┐   ┌────────┐   │
         │              ├───────────┤  Entity 1: │ Comp 1 │   │ Comp 2 │   │
         │              │           │     ───────┴────────┴───┴────────┘   │
         │              │           │                                      │
         │              │           │                         ┌────────┐   │
         │              │           │  Entity 2:              │ Comp 2 │   │
         │              │           │    ─────────────────────┴────────┘   │
         └──────────────┘           │                                      │
                                    └──────────────────────────────────────┘

```

# Usage
Currently only SQLite database has been developed as a `Database Resource` implementation. So this is all for a SQLite database. Feel free to add an issue for an new database resource.

Use the `DBQueryDerive` derive marco to generate the component mapper to map the database row into the component. Each attribute
on the struct is treated as column in the SQLite database.

Eg. Create a User component

```rust
use async_trait::async_trait;
use bevy::prelude::*;
use bevy_erm::*;
use bevy_erm_derive::DBQueryDerive;

#[derive(Component, sqlx::FromRow, DBQueryDerive)]
#[table_name = "users"]
pub struct User {
    pub name: String,
    pub age: i32,
}
```

For example if you have a webserver resource then you can pull messages from it.
Then start a new transaction to get the name of the user.

The `DatabaseResource` trait defines how the query access the actual database.
A Sqlite Database Connector using sqlx is already under `SqlxSqliteDatabaseResource`


```rust
use bevy::prelude::*;
use bevy_erm::*;

pub fn handle_webserver_events(
    db: Res<SqlxSqliteDatabaseResource>,
    webserver: Res<WebServer>, // Your own defined resource
) {
    while webserver.messages_to_handle() {
        let http_request = webserver.pop();
        let user_id = http_request.user_id;

        if let Some(request) = db.try_start_new_transaction() {
            let purchase_event = GetUserNameEvent {
                user_id,
                request,
            };

            // Forward the event on to the next system
            purchase_events.send(purchase_event);
        } else {
            // All connections are used 
            // wait for next round
            break;
        }
    }

}
```

Add a system to query users to handle the `GetUserNameEvent`.
Printing the users name. 

```rust
use bevy::prelude::*;
use bevy_erm::*;

pub fn print_user_names(
    users: DatabaseQuery<&User>,
    mut get_user_names_events: EventReader<GetUserNameEvent>,
) {
    for get_user_name_event in get_user_names_events.read() {
        let request = get_user_name_event.request;
        let user_id = get_user_name_event.user_id;

        let user = users.get(&(user_id, request))
            .await.unwrap();

        println!("{}", user.name);
    }

}
```

Setup the bevy App

```rust

fn main() {
    App::new()
        .set_runner(runner)
        .init_resource::<WebServer>()
        .add_event::<GetUserNameEvent>()
        // Add the bevy_erm plugin
        .add_plugins(EntityRelationMapperPlugin)
        // Specify how if components were created or updated how they 
        // would be flushed to the database
        .add_systems(PostUpdate, flush_component_to_db::<Option<&User>, SqlxSqliteDatabaseResource>)
        // Add the created systems above
        .add_systems(Update, handle_webserver_events)
        .add_systems(Update, print_user_names)
        // Add other systems 
        // ...
        .run();

}

```



# Examples
Found in ./examples