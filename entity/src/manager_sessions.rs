use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "manager_sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_type = "Text")]
    pub manager_id: String,
    #[sea_orm(column_type = "Text")]
    pub token: String,
    pub expires_at: chrono::NaiveDateTime, // Changed to NaiveDateTime to match your database
    #[sea_orm(column_name = "createdAt")]
    pub created_at: chrono::NaiveDateTime,
    #[sea_orm(column_name = "updatedAt")]
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::managers::Entity",
        from = "Column::ManagerId",
        to = "super::managers::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Managers,
}

impl Related<super::managers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Managers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}