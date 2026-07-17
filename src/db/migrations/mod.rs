use anyhow::Context;
use sqlx::PgPool;

mod base_schema;
mod compatibility;
mod indexes;
mod library_structure;
mod personalization;
mod presets;

pub(super) type MigrationTx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    let mut tx = pool.begin().await.context("failed to begin migration")?;

    base_schema::create_base_schema(&mut tx).await?;
    library_structure::create_library_structure_schema(&mut tx).await?;
    presets::create_presets_schema(&mut tx).await?;
    personalization::create_personalization_schema(&mut tx).await?;
    compatibility::upgrade_existing_schema(&mut tx).await?;
    indexes::create_indexes(&mut tx).await?;

    tx.commit().await.context("failed to commit migration")?;
    Ok(())
}
