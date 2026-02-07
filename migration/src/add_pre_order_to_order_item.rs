use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(OrderItem::Table)
                    .add_column(
                        ColumnDef::new(OrderItem::PreOrderOnPurchase)
                            .boolean()
                            .not_null()
                            .default(false)
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(OrderItem::Table)
                    .drop_column(OrderItem::PreOrderOnPurchase)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum OrderItem {
    #[sea_orm(iden = "OrderItem")]
    Table,
    #[sea_orm(iden = "id")]
    Id,
    #[sea_orm(iden = "orderId")]
    OrderId,
    #[sea_orm(iden = "productVariantId")]
    ProductVariantId,
    #[sea_orm(iden = "quantity")]
    Quantity,
    #[sea_orm(iden = "priceUsd")]
    PriceUsd,
    #[sea_orm(iden = "productTitle")]
    ProductTitle,
    #[sea_orm(iden = "variantName")]
    VariantName,
    #[sea_orm(iden = "pre_order_on_purchase")]
    PreOrderOnPurchase,
}
