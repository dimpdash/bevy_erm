

#[cfg(test)]
mod tests {
    use bevy_erm::*;
    use bevy_app::prelude::*;
    use bevy_utils::tracing::Event;
    use futures::executor::block_on;
    use sqlx::{prelude::*, query, SqliteConnection};
    use bevy_app::{App, AppExit};
    use bevy_ecs::{component::Component, event::EventReader};
    use bevy_ecs::event::Events;
    use bevy_ecs_macros::Event;
    use bevy_erm::{DBQueryDerive, DatabaseQuery};
    use async_trait::async_trait;
    use bevy_erm_core::database_query::{CustomDatabaseQuery, DatabaseTransaction};

    // have a sqlite database

    #[test]
    fn test() {
        block_on(async {
            // create a database
            let mut db = SqliteConnection::connect("sqlite::memory:").await.unwrap();

            // create a table
            query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
                .execute(&mut db)
                .await
                .unwrap();

            // insert a rows
            for i in 0..10 {
                query("INSERT INTO users (name) VALUES (?)")
                    .bind(format!("User {}", i))
                    .execute(&mut db)
                    .await
                    .unwrap();
            }

            {
                let app = App::new()
                    .add_plugins(EntityRelationMapperPlugin)
                    .add_event::<UserEvent>()
                    .add_systems(Update, get_user_names)
                    .set_runner(runner2)
                    ;
            }

        });
    }

    fn runner2(mut app: App) {
        loop {
            app.update();
    
            if !app
                .world
                .get_resource::<Events<AppExit>>()
                .unwrap()
                .is_empty()
            {
                break;
            }
        }
    }
    
    
    
    #[derive(Component, Debug, Default, Clone, sqlx::FromRow, DBQueryDerive)]
    #[table_name = "users"]
    struct User {
        name: String,
    }
    
    #[derive(Event)]
    struct UserEvent {
        user: DatabaseEntityId,
        request: RequestId,
    }

    struct UserEventResponse {
        user: String,
        request: RequestId,
    }
    
    fn get_user_names(users: DatabaseQuery<&User>, mut events: EventReader<UserEvent>, ) {

        block_on(async {
            for event in events.read() {
                let user = users.get(&(event.user, event.request)).await.unwrap();
            }


        })
    }



}

    
