use crate::{error::AppResult, models::LibraryWithRole, state::AppState};
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
        library_id,
        COUNT(*)::BIGINT AS storage_root_count,
        COUNT(*) FILTER (WHERE enabled)::BIGINT AS enabled_storage_root_count,
        (ARRAY_AGG(kind ORDER BY enabled DESC, created_at ASC))[1] AS primary_storage_kind,
        (ARRAY_AGG(canonical_uri ORDER BY enabled DESC, created_at ASC))[1] AS primary_storage_uri,
        (ARRAY_AGG(windows_unc_path ORDER BY enabled DESC, created_at ASC))[1] AS primary_storage_windows_path,
        (ARRAY_AGG(macos_smb_url ORDER BY enabled DESC, created_at ASC))[1] AS primary_storage_macos_path
    FROM storage_roots
    GROUP BY library_id
)
"#;

const LIBRARY_SELECT_COLUMNS: &str = r#"
SELECT
    l.id,
    l.display_name,
    l.description,
    l.icon_url,
    l.enabled,
    {role_expression} AS current_user_role,
    creator.display_name AS creator_name,
    COALESCE(ms.member_names, ARRAY[]::TEXT[]) AS member_names,
    COALESCE(ast.asset_count, 0) AS asset_count,
    COALESCE(fs.folder_count, 0) AS folder_count,
    COALESCE(ts.tag_count, 0) AS tag_count,
    COALESCE(ast.total_size_bytes, 0) AS total_size_bytes,
    COALESCE(ss.storage_root_count, 0) AS storage_root_count,
    COALESCE(ss.enabled_storage_root_count, 0) AS enabled_storage_root_count,
    ss.primary_storage_kind,
    ss.primary_storage_uri,
    ss.primary_storage_windows_path,
    ss.primary_storage_macos_path,
    l.created_by_user_id,
    l.created_at,
    l.updated_at
FROM libraries l
INNER JOIN users creator ON creator.id = l.created_by_user_id
"#;

pub async fn list_libraries_for_server_manager(
    state: &AppState,
    user_id: Uuid,
    fallback_role: &str,
) -> AppResult<Vec<LibraryWithRole>> {
    let sql = format!(
        "{}{}{}",
        LIBRARY_STATS_CTES,
        LIBRARY_SELECT_COLUMNS.replace("{role_expression}", "COALESCE(m.role, $2)"),
        r#"
        LEFT JOIN library_memberships m ON m.library_id = l.id AND m.user_id = $1
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
        LIBRARY_SELECT_COLUMNS.replace("{role_expression}", "m.role"),
        r#"
        INNER JOIN library_memberships m ON m.library_id = l.id
        LEFT JOIN member_stats ms ON ms.library_id = l.id
        LEFT JOIN asset_stats ast ON ast.library_id = l.id
        LEFT JOIN folder_stats fs ON fs.library_id = l.id
        LEFT JOIN tag_stats ts ON ts.library_id = l.id
        LEFT JOIN storage_stats ss ON ss.library_id = l.id
        WHERE l.deleted_at IS NULL AND m.user_id = $1
        ORDER BY l.display_name ASC
        "#
    );

    Ok(sqlx::query_as::<_, LibraryWithRole>(&sql)
        .bind(user_id)
        .fetch_all(&state.pool)
        .await?)
}
