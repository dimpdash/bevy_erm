use crate::*;

pub struct EntityRelationMapperPlugin;

impl Plugin for EntityRelationMapperPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FlushEvent>()
            .init_resource::<SqlxSqliteDatabaseResource>();
    }
}
