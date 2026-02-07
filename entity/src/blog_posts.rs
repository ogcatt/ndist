use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "blog_posts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    pub title: String,

    #[sea_orm(column_name = "subtitle", nullable)]
    pub subtitle: Option<String>,

    #[sea_orm(column_name = "thumbnailUrl", nullable)]
    pub thumbnail_url: Option<String>,

    #[sea_orm(column_name = "blogMd")]
    pub blog_md: String,

    #[sea_orm(column_name = "postedAt")]
    pub posted_at: DateTime,

    #[sea_orm(column_name = "updatedAt")]
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
