use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PreOrder::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PreOrder::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(PreOrder::OrderItemId)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PreOrder::ParentOrderId)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PreOrder::AddToEmailList)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(PreOrder::ShippingOption)
                            .enumeration(
                                Alias::new("shipping_option"),
                                [
                                    Alias::new("TRACKED"),
                                    Alias::new("EXPRESS"),
                                    Alias::new("TRACKED_US"),
                                ],
                            )
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PreOrder::OrderWeight)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(PreOrder::FulfilledAt)
                            .date_time()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(PreOrder::PreparedAt)
                            .date_time()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(PreOrder::TrackingUrl)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(PreOrder::Notes)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(PreOrder::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(PreOrder::UpdatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    // Foreign key constraints
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_preorder_order_item")
                            .from(PreOrder::Table, PreOrder::OrderItemId)
                            .to(OrderItem::Table, OrderItem::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_preorder_parent_order")
                            .from(PreOrder::Table, PreOrder::ParentOrderId)
                            .to(Order::Table, Order::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for better query performance
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_preorder_order_item_id")
                    .table(PreOrder::Table)
                    .col(PreOrder::OrderItemId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_preorder_parent_order_id")
                    .table(PreOrder::Table)
                    .col(PreOrder::ParentOrderId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_preorder_fulfilled_at")
                    .table(PreOrder::Table)
                    .col(PreOrder::FulfilledAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_preorder_created_at")
                    .table(PreOrder::Table)
                    .col(PreOrder::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PreOrder::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
#[sea_orm(iden = "PreOrder")]
enum PreOrder {
    #[sea_orm(iden = "PreOrder")]
    Table,
    #[sea_orm(iden = "id")]
    Id,
    #[sea_orm(iden = "orderItemId")]
    OrderItemId,
    #[sea_orm(iden = "parentOrderId")]
    ParentOrderId,
    #[sea_orm(iden = "addToEmailList")]
    AddToEmailList,
    #[sea_orm(iden = "shippingOption")]
    ShippingOption,
    #[sea_orm(iden = "orderWeight")]
    OrderWeight,
    #[sea_orm(iden = "fulfilledAt")]
    FulfilledAt,
    #[sea_orm(iden = "preparedAt")]
    PreparedAt,
    #[sea_orm(iden = "trackingUrl")]
    TrackingUrl,
    #[sea_orm(iden = "notes")]
    Notes,
    #[sea_orm(iden = "createdAt")]
    CreatedAt,
    #[sea_orm(iden = "updatedAt")]
    UpdatedAt,
}

#[derive(DeriveIden)]
#[sea_orm(iden = "OrderItem")]
enum OrderItem {
    #[sea_orm(iden = "OrderItem")]
    Table,
    #[sea_orm(iden = "id")]
    Id,
}

#[derive(DeriveIden)]
#[sea_orm(iden = "Order")]
enum Order {
    #[sea_orm(iden = "Order")]
    Table,
    #[sea_orm(iden = "id")]
    Id,
}
