use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BlogPosts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BlogPosts::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(BlogPosts::Title).text().not_null())
                    .col(ColumnDef::new(BlogPosts::Subtitle).text().null())
                    .col(ColumnDef::new(BlogPosts::ThumbnailUrl).text().null())
                    .col(ColumnDef::new(BlogPosts::BlogMd).text().not_null())
                    .col(ColumnDef::new(BlogPosts::PostedAt).timestamp().not_null())
                    .col(ColumnDef::new(BlogPosts::UpdatedAt).timestamp().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(BlogPosts::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum BlogPosts {
    Table,
    Id,
    Title,
    Subtitle,
    #[sea_orm(iden = "thumbnailUrl")]
    ThumbnailUrl,
    #[sea_orm(iden = "blogMd")]
    BlogMd,
    #[sea_orm(iden = "postedAt")]
    PostedAt,
    #[sea_orm(iden = "updatedAt")]
    UpdatedAt,
}
