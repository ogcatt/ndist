use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "invite_codes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_type = "Text", unique)]
    pub code: String,
    #[sea_orm(column_type = "Text")]
    pub group_id: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub api_key_id: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub used_by_user_id: Option<String>,
    #[sea_orm(nullable)]
    pub used_at: Option<DateTime>,
    pub is_revoked: bool,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::groups::Entity",
        from = "Column::GroupId",
        to = "super::groups::Column::Id",
        on_delete = "Cascade"
    )]
    Group,
}
impl ActiveModelBehavior for ActiveModel {}
