use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ProductVariants::Table)
                    .add_column(
                        ColumnDef::new(ProductVariants::PbxSku)
                            .text()
                            .null()
                    )
                    .add_column(
                        ColumnDef::new(ProductVariants::AdditionalThumbnailUrls)
                            .array(ColumnType::Text)
                            .null()
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ProductVariants::Table)
                    .drop_column(ProductVariants::PbxSku)
                    .drop_column(ProductVariants::AdditionalThumbnailUrls)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum ProductVariants {
    Table,
    #[sea_orm(iden = "pbxSku")]
    PbxSku,
    #[sea_orm(iden = "additionalThumbnailUrls")]
    AdditionalThumbnailUrls,
}
