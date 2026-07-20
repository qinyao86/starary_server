use crate::{
    error::AppResult,
    models::{LibraryStatusRecord, LibraryWithRole},
    state::AppState,
};
use uuid::Uuid;

const LIBRARY_STATS_CTES: &str = r#"
WITH member_stats AS (
    SELECT
        m.library_id,
        ARRAY_AGG(u.display_name ORDER BY u.display_name) AS member_names
    FROM library_memberships m
    INNER JOIN users u ON u.id = m.user_id
    GROUP BY m.library_id
),
library_manager_stats AS (
    SELECT
        m.library_id,
        ARRAY_AGG(u.display_name ORDER BY u.display_name) AS library_manager_names,
        ARRAY_AGG(u.id ORDER BY u.display_name) AS library_manager_user_ids,
        ARRAY_AGG(u.avatar_key ORDER BY u.display_name) AS library_manager_avatar_keys
    FROM library_memberships m
    INNER JOIN users u ON u.id = m.user_id
    WHERE m.role IN ('owner', 'admin', 'library_manager')
    GROUP BY m.library_id
),
asset_stats AS (
    SELECT
        library_id,
        COUNT(*) FILTER (WHERE deleted_at IS NULL)::BIGINT AS asset_count,
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
folder_stats AS (
    SELECT
        library_id,
        COUNT(*)::BIGINT AS folder_count
    FROM folders
    GROUP BY library_id
),
tag_stats AS (
    SELECT
        library_id,
        COUNT(*)::BIGINT AS tag_count
    FROM tags
    GROUP BY library_id
),
storage_stats AS (
    SELECT
        sr.library_id,
        COUNT(*)::BIGINT AS storage_root_count,
        COUNT(*) FILTER (WHERE sr.enabled AND sc.enabled)::BIGINT AS enabled_storage_root_count,
        (ARRAY_AGG(sc.kind ORDER BY sr.enabled DESC, sr.created_at ASC))[1] AS primary_storage_kind,
        (ARRAY_AGG(sc.id ORDER BY sr.enabled DESC, sr.created_at ASC))[1] AS primary_storage_connection_id,
        (ARRAY_AGG(sc.name ORDER BY sr.enabled DESC, sr.created_at ASC))[1] AS primary_storage_connection_name,
        (ARRAY_AGG(sr.namespace ORDER BY sr.enabled DESC, sr.created_at ASC))[1] AS primary_storage_namespace,
        (ARRAY_AGG(sr.canonical_uri ORDER BY sr.enabled DESC, sr.created_at ASC))[1] AS primary_storage_uri,
        (ARRAY_AGG(sr.windows_unc_path ORDER BY sr.enabled DESC, sr.created_at ASC))[1] AS primary_storage_windows_path,
        (ARRAY_AGG(sr.macos_smb_url ORDER BY sr.enabled DESC, sr.created_at ASC))[1] AS primary_storage_macos_path
    FROM storage_roots sr
    INNER JOIN storage_connections sc ON sc.id = sr.storage_connection_id
    GROUP BY sr.library_id
)
"#;

const LIBRARY_SELECT_COLUMNS: &str = r#"
SELECT
    l.id,
    l.display_name,
    l.icon_url,
    l.enabled,
    l.access_mode,
    l.storage_locked_at,
    {role_expression} AS current_user_role,
    {member_expression} AS is_member,
    COALESCE(lms.library_manager_names, ARRAY[]::TEXT[]) AS library_manager_names,
    COALESCE(lms.library_manager_user_ids, ARRAY[]::UUID[]) AS library_manager_user_ids,
    COALESCE(lms.library_manager_avatar_keys, ARRAY[]::TEXT[]) AS library_manager_avatar_keys,
    COALESCE(ms.member_names, ARRAY[]::TEXT[]) AS member_names,
    COALESCE(ast.asset_count, 0) AS asset_count,
    COALESCE(fs.folder_count, 0) AS folder_count,
    COALESCE(ts.tag_count, 0) AS tag_count,
    COALESCE(ast.total_size_bytes, 0) AS total_size_bytes,
    COALESCE(ss.storage_root_count, 0) AS storage_root_count,
    COALESCE(ss.enabled_storage_root_count, 0) AS enabled_storage_root_count,
    ss.primary_storage_kind,
    ss.primary_storage_connection_id,
    ss.primary_storage_connection_name,
    ss.primary_storage_namespace,
    ss.primary_storage_uri,
    ss.primary_storage_windows_path,
    ss.primary_storage_macos_path,
    l.created_by_user_id,
    l.created_at,
    l.updated_at
FROM libraries l
"#;

pub async fn list_libraries_for_server_manager(
    state: &AppState,
    user_id: Uuid,
    fallback_role: &str,
) -> AppResult<Vec<LibraryWithRole>> {
    let sql = format!(
        "{}{}{}",
        LIBRARY_STATS_CTES,
        LIBRARY_SELECT_COLUMNS
            .replace("{role_expression}", "COALESCE(m.role, $2)")
            .replace("{member_expression}", "m.user_id IS NOT NULL"),
        r#"
        LEFT JOIN library_memberships m ON m.library_id = l.id AND m.user_id = $1
        LEFT JOIN library_manager_stats lms ON lms.library_id = l.id
        LEFT JOIN member_stats ms ON ms.library_id = l.id
        LEFT JOIN asset_stats ast ON ast.library_id = l.id
        LEFT JOIN folder_stats fs ON fs.library_id = l.id
        LEFT JOIN tag_stats ts ON ts.library_id = l.id
        LEFT JOIN storage_stats ss ON ss.library_id = l.id
        WHERE l.deleted_at IS NULL
        ORDER BY l.display_name ASC
        "#
    );

    Ok(sqlx::query_as::<_, LibraryWithRole>(&sql)
        .bind(user_id)
        .bind(fallback_role)
        .fetch_all(&state.pool)
        .await?)
}

pub async fn list_libraries_for_member(
    state: &AppState,
    user_id: Uuid,
) -> AppResult<Vec<LibraryWithRole>> {
    let sql = format!(
        "{}{}{}",
        LIBRARY_STATS_CTES,
        LIBRARY_SELECT_COLUMNS
            .replace("{role_expression}", "m.role")
            .replace("{member_expression}", "m.user_id IS NOT NULL"),
        r#"
        LEFT JOIN library_memberships m ON m.library_id = l.id AND m.user_id = $1
        LEFT JOIN library_manager_stats lms ON lms.library_id = l.id
        LEFT JOIN member_stats ms ON ms.library_id = l.id
        LEFT JOIN asset_stats ast ON ast.library_id = l.id
        LEFT JOIN folder_stats fs ON fs.library_id = l.id
        LEFT JOIN tag_stats ts ON ts.library_id = l.id
        LEFT JOIN storage_stats ss ON ss.library_id = l.id
        WHERE l.deleted_at IS NULL
          AND (m.user_id IS NOT NULL OR (l.access_mode = 'public' AND l.enabled))
        ORDER BY l.display_name ASC
        "#
    );

    Ok(sqlx::query_as::<_, LibraryWithRole>(&sql)
        .bind(user_id)
        .fetch_all(&state.pool)
        .await?)
}

pub async fn list_libraries_for_library_manager(
    state: &AppState,
    user_id: Uuid,
) -> AppResult<Vec<LibraryWithRole>> {
    let sql = format!(
        "{}{}{}",
        LIBRARY_STATS_CTES,
        LIBRARY_SELECT_COLUMNS
            .replace("{role_expression}", "m.role")
            .replace("{member_expression}", "TRUE"),
        r#"
        INNER JOIN library_memberships m ON m.library_id = l.id
        LEFT JOIN library_manager_stats lms ON lms.library_id = l.id
        LEFT JOIN member_stats ms ON ms.library_id = l.id
        LEFT JOIN asset_stats ast ON ast.library_id = l.id
        LEFT JOIN folder_stats fs ON fs.library_id = l.id
        LEFT JOIN tag_stats ts ON ts.library_id = l.id
        LEFT JOIN storage_stats ss ON ss.library_id = l.id
        WHERE l.deleted_at IS NULL
          AND m.user_id = $1
          AND m.role IN ('owner', 'admin', 'library_manager')
        ORDER BY l.display_name ASC
        "#
    );

    Ok(sqlx::query_as::<_, LibraryWithRole>(&sql)
        .bind(user_id)
        .fetch_all(&state.pool)
        .await?)
}

pub async fn list_library_statuses_for_server_manager(
    state: &AppState,
) -> AppResult<Vec<LibraryStatusRecord>> {
    Ok(sqlx::query_as::<_, LibraryStatusRecord>(
        r#"
        SELECT id, enabled, updated_at
        FROM libraries
        WHERE deleted_at IS NULL
        ORDER BY id ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?)
}

pub async fn list_library_statuses_for_library_manager(
    state: &AppState,
    user_id: Uuid,
) -> AppResult<Vec<LibraryStatusRecord>> {
    Ok(sqlx::query_as::<_, LibraryStatusRecord>(
        r#"
        SELECT l.id, l.enabled, l.updated_at
        FROM libraries l
        INNER JOIN library_memberships m ON m.library_id = l.id
        WHERE l.deleted_at IS NULL
          AND m.user_id = $1
          AND m.role IN ('owner', 'admin', 'library_manager')
        ORDER BY l.id ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?)
}

pub async fn list_library_statuses_for_member(
    state: &AppState,
    user_id: Uuid,
) -> AppResult<Vec<LibraryStatusRecord>> {
    Ok(sqlx::query_as::<_, LibraryStatusRecord>(
        r#"
        SELECT l.id, l.enabled, l.updated_at
        FROM libraries l
        LEFT JOIN library_memberships m ON m.library_id = l.id AND m.user_id = $1
        WHERE l.deleted_at IS NULL
          AND (m.user_id IS NOT NULL OR (l.access_mode = 'public' AND l.enabled))
        ORDER BY l.id ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?)
}
