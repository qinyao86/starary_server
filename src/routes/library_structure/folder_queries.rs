use crate::{
    error::{AppError, AppResult},
    models::FolderRecord,
    state::AppState,
};

pub async fn query_folders(state: &AppState, library_id: &str) -> AppResult<Vec<FolderRecord>> {
    Ok(sqlx::query_as::<_, FolderRecord>(
        r#"
        SELECT
            f.id,
            f.parent_id,
            f.name,
            f.description,
            f.icon,
            f.color,
            COUNT(counted_assets.id)::BIGINT AS asset_count,
            f.cover_asset_id,
            CASE
                WHEN a.id IS NULL THEN NULL
                ELSE jsonb_build_object(
                    'id', a.id::text,
                    'name', a.name,
                    'assetKind', a.asset_kind,
                    'storedPath', COALESCE(a.relative_path, a.storage_key, ''),
                    'thumbnailPath', a.metadata->>'thumbnailPath'
                )
            END AS cover_asset,
            f.smart_import_id,
            f.sort_order,
            f.created_by_user_id,
            f.updated_by_user_id,
            f.created_at,
            f.updated_at
        FROM folders f
        LEFT JOIN asset_folders af ON af.folder_id = f.id
        LEFT JOIN assets counted_assets
            ON counted_assets.id = af.asset_id
           AND counted_assets.library_id = f.library_id
           AND counted_assets.deleted_at IS NULL
        LEFT JOIN assets a ON a.id = f.cover_asset_id AND a.deleted_at IS NULL
        WHERE f.library_id = $1
        GROUP BY f.id, a.id
        ORDER BY f.sort_order ASC, f.created_at ASC
        "#,
    )
    .bind(library_id)
    .fetch_all(&state.pool)
    .await?)
}

pub async fn ensure_parent_folder(
    state: &AppState,
    library_id: &str,
    parent_id: Option<&str>,
) -> AppResult<()> {
    let Some(parent_id) = parent_id else {
        return Ok(());
    };

    let exists: Option<String> =
        sqlx::query_scalar("SELECT id FROM folders WHERE library_id = $1 AND id = $2")
            .bind(library_id)
            .bind(parent_id)
            .fetch_optional(&state.pool)
            .await?;

    if exists.is_none() {
        return Err(AppError::NotFound("parent folder not found".to_string()));
    }

    Ok(())
}

pub async fn next_folder_sort_order(
    state: &AppState,
    library_id: &str,
    parent_id: Option<&str>,
) -> AppResult<i64> {
    Ok(sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COALESCE(MAX(sort_order), 0) + 1000
        FROM folders
        WHERE library_id = $1
          AND (($2::text IS NULL AND parent_id IS NULL) OR parent_id = $2)
        "#,
    )
    .bind(library_id)
    .bind(parent_id)
    .fetch_one(&state.pool)
    .await?)
}

pub async fn query_folder_name(
    state: &AppState,
    library_id: &str,
    folder_id: &str,
) -> AppResult<String> {
    sqlx::query_scalar("SELECT name FROM folders WHERE library_id = $1 AND id = $2")
        .bind(library_id)
        .bind(folder_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("folder not found".to_string()))
}

pub async fn query_folder_branch_ids(
    state: &AppState,
    library_id: &str,
    folder_id: &str,
) -> AppResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        r#"
        WITH RECURSIVE folder_branch AS (
            SELECT id
            FROM folders
            WHERE library_id = $1 AND id = $2
            UNION ALL
            SELECT child.id
            FROM folders child
            INNER JOIN folder_branch parent ON child.parent_id = parent.id
            WHERE child.library_id = $1
        )
        SELECT id FROM folder_branch
        "#,
    )
    .bind(library_id)
    .bind(folder_id)
    .fetch_all(&state.pool)
    .await?)
}

pub async fn query_folder_edit_state(
    state: &AppState,
    library_id: &str,
    folder_id: &str,
) -> AppResult<FolderEditState> {
    sqlx::query_as::<_, FolderEditState>(
        r#"
        SELECT name, description, icon, color, cover_asset_id, smart_import_id
        FROM folders
        WHERE library_id = $1 AND id = $2
        "#,
    )
    .bind(library_id)
    .bind(folder_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("folder not found".to_string()))
}

pub async fn ensure_folder_exists_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    library_id: &str,
    folder_id: &str,
) -> AppResult<()> {
    let exists: Option<String> =
        sqlx::query_scalar("SELECT id FROM folders WHERE library_id = $1 AND id = $2")
            .bind(library_id)
            .bind(folder_id)
            .fetch_optional(&mut **tx)
            .await?;

    if exists.is_none() {
        return Err(AppError::NotFound("folder not found".to_string()));
    }

    Ok(())
}

pub async fn query_sibling_folder_ids_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    library_id: &str,
    parent_id: Option<&str>,
) -> AppResult<Vec<String>> {
    Ok(sqlx::query_scalar::<_, String>(
        r#"
        SELECT id
        FROM folders
        WHERE library_id = $1
          AND (($2::text IS NULL AND parent_id IS NULL) OR parent_id = $2)
        ORDER BY sort_order ASC, created_at ASC
        "#,
    )
    .bind(library_id)
    .bind(parent_id)
    .fetch_all(&mut **tx)
    .await?)
}

pub async fn ensure_folder_can_move(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    library_id: &str,
    folder_id: &str,
    parent_id: Option<&str>,
) -> AppResult<()> {
    let Some(parent_id) = parent_id else {
        return Ok(());
    };
    if parent_id == folder_id {
        return Err(AppError::BadRequest(
            "folder cannot be moved into itself".to_string(),
        ));
    }

    let is_descendant: bool = sqlx::query_scalar(
        r#"
        WITH RECURSIVE folder_branch AS (
            SELECT id
            FROM folders
            WHERE library_id = $1 AND id = $2
            UNION ALL
            SELECT child.id
            FROM folders child
            INNER JOIN folder_branch branch ON child.parent_id = branch.id
            WHERE child.library_id = $1
        )
        SELECT EXISTS(SELECT 1 FROM folder_branch WHERE id = $3)
        "#,
    )
    .bind(library_id)
    .bind(folder_id)
    .bind(parent_id)
    .fetch_one(&mut **tx)
    .await?;

    if is_descendant {
        return Err(AppError::BadRequest(
            "folder cannot be moved into its child".to_string(),
        ));
    }

    Ok(())
}

#[derive(sqlx::FromRow)]
pub struct FolderEditState {
    pub name: String,
    pub description: String,
    pub icon: String,
    pub color: String,
    pub cover_asset_id: Option<String>,
    pub smart_import_id: Option<String>,
}
