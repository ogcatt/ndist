use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the existing foreign key constraint
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_address_order_id")
                    .table(Address::Table)
                    .to_owned(),
            )
            .await?;

        // Create the new foreign key constraint with CASCADE delete
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_address_order_id")
                    .from(Address::Table, Address::OrderId)
                    .to(Order::Table, Order::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the CASCADE foreign key constraint
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_address_order_id")
                    .table(Address::Table)
                    .to_owned(),
            )
            .await?;

        // Restore the original RESTRICT constraint
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_address_order_id")
                    .from(Address::Table, Address::OrderId)
                    .to(Order::Table, Order::Id)
                    .on_delete(ForeignKeyAction::Restrict)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Order {
    #[sea_orm(iden = "Order")]
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Address {
    #[sea_orm(iden = "Address")]
    Table,
    #[sea_orm(iden = "orderId")]
    OrderId,
}
