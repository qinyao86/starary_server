use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    models::{LibraryWithRole, Role, TeamLibraryRecord},
    routes::access::ensure_library_manager,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLibraryRequest {
    name: String,
    description: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLibraryRequest {
    name: String,
    description: Option<String>,
}

pub async fn list_libraries(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<LibraryWithRole>>> {
    let libraries = if user.role.can_manage_server() {
        sqlx::query_as::<_, LibraryWithRole>(
            r#"
            WITH member_stats AS (
                SELECT
                    m.library_id,
                    ARRAY_AGG(u.display_name ORDER BY u.display_name) AS member_names
                FROM library_memberships m
                INNER JOIN users u ON u.id = m.user_id
                GROUP BY m.library_id
            ),
            asset_stats AS (
                SELECT
                    library_id,
                    COUNT(*) FILTER (WHERE deleted_at IS NULL)::BIGINT AS asset_count,
                    COUNT(*) FILTER (
                        WHERE deleted_at IS NULL
                        AND (asset_kind = 'folder' OR asset_kind = 'package')
                    )::BIGINT AS folder_count,
                    COALESCE(SUM(
                        CASE
                            WHEN deleted_at IS NULL AND (metadata->>'sizeBytes') ~ '^[0-9]+$' THEN (metadata->>'sizeBytes')::BIGINT
                            WHEN deleted_at IS NULL AND (metadata->>'fileSize') ~ '^[0-9]+$' THEN (metadata->>'fileSize')::BIGINT
                            WHEN deleted_at IS NULL AND (metadata->>'size') ~ '^[0-9]+$' THEN (metadata->>'size')::BIGINT
                            ELSE 0
                        END
                    ), 0)::BIGINT AS total_size_bytes
                FROM assets
                GROUP BY library_id
            ),
            tag_stats AS (
                SELECT
                    assets.library_id,
                    COUNT(DISTINCT tag.value)::BIGINT AS tag_count
                FROM assets
                CROSS JOIN LATERAL jsonb_array_elements_text(
                    CASE
                        WHEN jsonb_typeof(metadata->'tags') = 'array' THEN metadata->'tags'
                        ELSE '[]'::jsonb
                    END
                ) AS tag(value)
                WHERE assets.deleted_at IS NULL
                GROUP BY assets.library_id
            )
            SELECT
                l.id,
                l.name,
                l.description,
                COALESCE(m.role, $2) AS current_user_role,
                creator.display_name AS creator_name,
                COALESCE(ms.member_names, ARRAY[]::TEXT[]) AS member_names,
                COALESCE(ast.asset_count, 0) AS asset_count,
                COALESCE(ast.folder_count, 0) AS folder_count,
                COALESCE(ts.tag_count, 0) AS tag_count,
                COALESCE(ast.total_size_bytes, 0) AS total_size_bytes,
                l.created_by_user_id,
                l.created_at,
                l.updated_at
            FROM team_libraries l
            INNER JOIN users creator ON creator.id = l.created_by_user_id
            LEFT JOIN library_memberships m ON m.library_id = l.id AND m.user_id = $1
            LEFT JOIN member_stats ms ON ms.library_id = l.id
            LEFT JOIN asset_stats ast ON ast.library_id = l.id
            LEFT JOIN tag_stats ts ON ts.library_id = l.id
            WHERE l.deleted_at IS NULL
            ORDER BY l.name ASC
            "#,
        )
        .bind(user.id)
        .bind(user.role.as_str())
        .fetch_all(&state.pool)
        .await?
    } else {
        sqlx::query_as::<_, LibraryWithRole>(
            r#"
            WITH member_stats AS (
                SELECT
                    m.library_id,
                    ARRAY_AGG(u.display_name ORDER BY u.display_name) AS member_names
                FROM library_memberships m
                INNER JOIN users u ON u.id = m.user_id
                GROUP BY m.library_id
            ),
            asset_stats AS (
                SELECT
                    library_id,
                    COUNT(*) FILTER (WHERE deleted_at IS NULL)::BIGINT AS asset_count,
                    COUNT(*) FILTER (
                        WHERE deleted_at IS NULL
                        AND (asset_kind = 'folder' OR asset_kind = 'package')
                    )::BIGINT AS folder_count,
                    COALESCE(SUM(
                        CASE
                            WHEN deleted_at IS NULL AND (metadata->>'sizeBytes') ~ '^[0-9]+$' THEN (metadata->>'sizeBytes')::BIGINT
                            WHEN deleted_at IS NULL AND (metadata->>'fileSize') ~ '^[0-9]+$' THEN (metadata->>'fileSize')::BIGINT
                            WHEN deleted_at IS NULL AND (metadata->>'size') ~ '^[0-9]+$' THEN (metadata->>'size')::BIGINT
                            ELSE 0
                        END
                    ), 0)::BIGINT AS total_size_bytes
                FROM assets
                GROUP BY library_id
            ),
            tag_stats AS (
                SELECT
                    assets.library_id,
                    COUNT(DISTINCT tag.value)::BIGINT AS tag_count
                FROM assets
                CROSS JOIN LATERAL jsonb_array_elements_text(
                    CASE
                        WHEN jsonb_typeof(metadata->'tags') = 'array' THEN metadata->'tags'
                        ELSE '[]'::jsonb
                    END
                ) AS tag(value)
                WHERE assets.deleted_at IS NULL
                GROUP BY assets.library_id
            )
            SELECT
                l.id,
                l.name,
                l.description,
                m.role AS current_user_role,
                creator.display_name AS creator_name,
                COALESCE(ms.member_names, ARRAY[]::TEXT[]) AS member_names,
                COALESCE(ast.asset_count, 0) AS asset_count,
                COALESCE(ast.folder_count, 0) AS folder_count,
                COALESCE(ts.tag_count, 0) AS tag_count,
                COALESCE(ast.total_size_bytes, 0) AS total_size_bytes,
                l.created_by_user_id,
                l.created_at,
                l.updated_at
            FROM team_libraries l
            INNER JOIN users creator ON creator.id = l.created_by_user_id
            INNER JOIN library_memberships m ON m.library_id = l.id
            LEFT JOIN member_stats ms ON ms.library_id = l.id
            LEFT JOIN asset_stats ast ON ast.library_id = l.id
            LEFT JOIN tag_stats ts ON ts.library_id = l.id
            WHERE l.deleted_at IS NULL AND m.user_id = $1
            ORDER BY l.name ASC
            "#,
        )
        .bind(user.id)
        .fetch_all(&state.pool)
        .await?
    };

    Ok(Json(libraries))
}

pub async fn create_library(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<CreateLibraryRequest>,
) -> AppResult<Json<TeamLibraryRecord>> {
    if !user.role.can_create_library() {
        return Err(AppError::Forbidden);
    }

    let name = request.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("library name is required".to_string()));
    }

    let library_id = Uuid::new_v4();
    let mut tx = state.pool.begin().await?;

    let library = sqlx::query_as::<_, TeamLibraryRecord>(
        r#"
        INSERT INTO team_libraries (id, name, description, created_by_user_id)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, description, created_by_user_id, created_at, updated_at
        "#,
    )
    .bind(library_id)
    .bind(name)
    .bind(request.description)
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO library_memberships (library_id, user_id, role)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(library_id)
    .bind(user.id)
    .bind(Role::LibraryManager.as_str())
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id)
        VALUES ($1, $2, $3, 'library.created', 'library', $2)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(library_id)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(library))
}

pub async fn update_library(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<Uuid>,
    Json(request): Json<UpdateLibraryRequest>,
) -> AppResult<Json<TeamLibraryRecord>> {
    ensure_library_manager(&state, &user, library_id).await?;

    let name = request.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("library name is required".to_string()));
    }

    let mut tx = state.pool.begin().await?;
    let library = sqlx::query_as::<_, TeamLibraryRecord>(
        r#"
        UPDATE team_libraries
        SET name = $2, description = $3, updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        RETURNING id, name, description, created_by_user_id, created_at, updated_at
        "#,
    )
    .bind(library_id)
    .bind(name)
    .bind(request.description)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("library not found".to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id)
        VALUES ($1, $2, $3, 'library.updated', 'library', $2)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(library_id)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(library))
}

pub async fn delete_library(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    ensure_library_manager(&state, &user, library_id).await?;

    let mut tx = state.pool.begin().await?;
    let deleted = sqlx::query(
        r#"
        UPDATE team_libraries
        SET deleted_at = NOW(), updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(library_id)
    .execute(&mut *tx)
    .await?;

    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound("library not found".to_string()));
    }

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id)
        VALUES ($1, $2, $3, 'library.deleted', 'library', $2)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(library_id)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}
