use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        // 1. Create new PostgreSQL enum for stock location shipping methods
        conn.execute_unprepared(
            r#"CREATE TYPE "stock_location_shipping_method" AS ENUM ('MANUAL', 'FLAT_RATE')"#,
        )
        .await?;

        // 2. Create stock_locations table
        conn.execute_unprepared(
            r#"
            CREATE TABLE IF NOT EXISTS stock_locations (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                "shippingMethod" "stock_location_shipping_method" NOT NULL DEFAULT 'MANUAL',
                "flatRateUsd" DOUBLE PRECISION,
                country TEXT,
                "createdAt" TIMESTAMP NOT NULL DEFAULT NOW(),
                "updatedAt" TIMESTAMP NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .await?;

        // 3. Create stock_location_quantities table
        //    Tracks per-location stock quantity for each stock item.
        //    UNIQUE constraint ensures one record per (item, location) pair.
        conn.execute_unprepared(
            r#"
            CREATE TABLE IF NOT EXISTS stock_location_quantities (
                id TEXT PRIMARY KEY,
                "stockItemId" TEXT NOT NULL REFERENCES stock_items(id) ON UPDATE CASCADE ON DELETE CASCADE,
                "stockLocationId" TEXT NOT NULL REFERENCES stock_locations(id) ON UPDATE CASCADE ON DELETE CASCADE,
                quantity INTEGER NOT NULL DEFAULT 0,
                "createdAt" TIMESTAMP NOT NULL DEFAULT NOW(),
                "updatedAt" TIMESTAMP NOT NULL DEFAULT NOW(),
                UNIQUE ("stockItemId", "stockLocationId")
            )
            "#,
        )
        .await?;

        // 4. Create stock_quantity_adjustments table
        //    Immutable audit log: each row records a quantity change with a required note.
        conn.execute_unprepared(
            r#"
            CREATE TABLE IF NOT EXISTS stock_quantity_adjustments (
                id TEXT PRIMARY KEY,
                "stockLocationQuantityId" TEXT NOT NULL REFERENCES stock_location_quantities(id) ON UPDATE CASCADE ON DELETE CASCADE,
                delta INTEGER NOT NULL,
                note TEXT NOT NULL,
                "adjustedBy" TEXT,
                "createdAt" TIMESTAMP NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .await?;

        // 5. Add stockLocationId to customer_baskets
        //    Nullable: set when the customer selects a stock location for their session.
        conn.execute_unprepared(
            r#"ALTER TABLE customer_baskets ADD COLUMN "stockLocationId" TEXT REFERENCES stock_locations(id) ON UPDATE CASCADE ON DELETE SET NULL"#,
        )
        .await?;

        // 6. Modify stock_backorder_active_reduce
        //    - Drop stockUnit column (units are now always integers)
        //    - Change reductionQuantity from float to integer
        //    - Add stockLocationId to track which location the reduction applies to
        conn.execute_unprepared(
            r#"ALTER TABLE stock_backorder_active_reduce DROP COLUMN IF EXISTS "stockUnit""#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE stock_backorder_active_reduce ALTER COLUMN "reductionQuantity" TYPE INTEGER USING "reductionQuantity"::INTEGER"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE stock_backorder_active_reduce ADD COLUMN "stockLocationId" TEXT REFERENCES stock_locations(id) ON UPDATE CASCADE ON DELETE SET NULL"#,
        )
        .await?;

        // 7. Modify stock_preorder_active_reduce (same changes as backorder)
        conn.execute_unprepared(
            r#"ALTER TABLE stock_preorder_active_reduce DROP COLUMN IF EXISTS "stockUnit""#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE stock_preorder_active_reduce ALTER COLUMN "reductionQuantity" TYPE INTEGER USING "reductionQuantity"::INTEGER"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE stock_preorder_active_reduce ADD COLUMN "stockLocationId" TEXT REFERENCES stock_locations(id) ON UPDATE CASCADE ON DELETE SET NULL"#,
        )
        .await?;

        // 8. Modify stock_items
        //    - Drop unit column (no longer needed; units are always integer multiples)
        //    - Drop isContainer column (items can no longer contain other items)
        //    - Change warningQuantity from float to integer
        conn.execute_unprepared(
            r#"ALTER TABLE stock_items DROP COLUMN IF EXISTS unit"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE stock_items DROP COLUMN IF EXISTS "isContainer""#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE stock_items ALTER COLUMN "warningQuantity" TYPE INTEGER USING ("warningQuantity"::INTEGER)"#,
        )
        .await?;

        // 9. Modify product_variant_stock_item_relations
        //    - Drop stockUnitOnCreation column (units are always integer multiples now)
        //    - Change quantity from float to integer
        conn.execute_unprepared(
            r#"ALTER TABLE product_variant_stock_item_relations DROP COLUMN IF EXISTS "stockUnitOnCreation""#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE product_variant_stock_item_relations ALTER COLUMN quantity TYPE INTEGER USING quantity::INTEGER"#,
        )
        .await?;

        // 10. Drop tables that are replaced by the new location-based system
        //     stock_active_reduce: replaced by stock_quantity_adjustments audit log
        //     stock_batches: replaced by stock_location_quantities + adjustments
        //     stock_item_relations: item containment is removed from the system
        conn.execute_unprepared(
            r#"DROP TABLE IF EXISTS stock_active_reduce CASCADE"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"DROP TABLE IF EXISTS stock_batches CASCADE"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"DROP TABLE IF EXISTS stock_item_relations CASCADE"#,
        )
        .await?;

        // 11. Drop old enum types (all columns using them have been removed above)
        conn.execute_unprepared(r#"DROP TYPE IF EXISTS "stock_unit" CASCADE"#)
            .await?;
        conn.execute_unprepared(r#"DROP TYPE IF EXISTS "stock_batch_status" CASCADE"#)
            .await?;
        conn.execute_unprepared(r#"DROP TYPE IF EXISTS "stock_batch_location" CASCADE"#)
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        // Drop new tables (in reverse dependency order)
        conn.execute_unprepared(
            r#"DROP TABLE IF EXISTS stock_quantity_adjustments CASCADE"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"DROP TABLE IF EXISTS stock_location_quantities CASCADE"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"DROP TABLE IF EXISTS stock_locations CASCADE"#,
        )
        .await?;

        // Remove columns added to existing tables
        conn.execute_unprepared(
            r#"ALTER TABLE customer_baskets DROP COLUMN IF EXISTS "stockLocationId""#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE stock_backorder_active_reduce DROP COLUMN IF EXISTS "stockLocationId""#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE stock_preorder_active_reduce DROP COLUMN IF EXISTS "stockLocationId""#,
        )
        .await?;

        // Drop new enum
        conn.execute_unprepared(
            r#"DROP TYPE IF EXISTS "stock_location_shipping_method""#,
        )
        .await?;

        Ok(())
    }
}
