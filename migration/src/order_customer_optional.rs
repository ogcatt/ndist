use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // First, drop the foreign key constraint
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .table(Order::Table)
                    .name("Order_customerId_fkey") // Adjust this name based on your actual FK constraint name
                    .to_owned(),
            )
            .await?;

        // Modify the column to be nullable
        manager
            .alter_table(
                Table::alter()
                    .table(Order::Table)
                    .modify_column(
                        ColumnDef::new(Order::CustomerId)
                            .text()
                            .null()
                    )
                    .to_owned(),
            )
            .await?;

        // Re-add the foreign key constraint (optional, but maintains referential integrity)
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("Order_customerId_fkey")
                    .from(Order::Table, Order::CustomerId)
                    .to(Customers::Table, Customers::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::Restrict)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the foreign key constraint
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .table(Order::Table)
                    .name("Order_customerId_fkey")
                    .to_owned(),
            )
            .await?;

        // Modify the column back to NOT NULL
        // Note: This will fail if there are NULL values in the column
        manager
            .alter_table(
                Table::alter()
                    .table(Order::Table)
                    .modify_column(
                        ColumnDef::new(Order::CustomerId)
                            .text()
                            .not_null()
                    )
                    .to_owned(),
            )
            .await?;

        // Re-add the foreign key constraint
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("Order_customerId_fkey")
                    .from(Order::Table, Order::CustomerId)
                    .to(Customers::Table, Customers::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::Restrict)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Order {
    #[sea_orm(iden = "Order")]
    Table,
    #[sea_orm(iden = "customerId")]
    CustomerId,
}

#[derive(DeriveIden)]
enum Customers {
    Table,
    Id,
}
