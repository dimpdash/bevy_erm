//test
#[cfg(test)]
mod tests {
    use bevy_erm_derive::DBQueryDerive;
    use bevy_erm_core::ComponentMapper;
    use sqlx::prelude::FromRow;
    use async_trait::async_trait;
    use sqlx;


    pub trait Easy {
        fn easy(&self) -> i32;
    }

    #[derive(DBQueryDerive, FromRow)]
    #[table_name = "test"]
    struct Test {
        id: i32,
        name: String,
        price: f32,
    }

    fn test() {
        // let t = Test {
        //     id: 1,
        //     name: "test".to_string(),
        //     price: 1.0,
        // };
        // let e = bevy_erm::DatabaseEntityId::new(1);
        // let mut tr = bevy_erm::AnyDatabaseResource::Transaction::new();
        // let r = Test::get(&mut tr, &e);
        // match r {
        //     Ok(_) => (),
        //     Err(_) => (),
        // }
    }
}