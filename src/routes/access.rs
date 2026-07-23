use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    models::Role,
    state::AppState,
};

pub async fn ensure_library_membership(
    state: &AppState,
    user: &AuthUser,
    library_id: &str,
) -> AppResult<Role> {
    if user.role.can_manage_server() {
        let exists: Option<String> =
            sqlx::query_scalar("SELECT id FROM libraries WHERE id = $1 AND deleted_at IS NULL")
                .bind(library_id)
                .fetch_optional(&state.pool)
                .await?;

        if exists.is_none() {
            return Err(AppError::NotFound("library not found".to_string()));
        }

        return Ok(user.role);
    }

    let role_value: Option<String> = sqlx::query_scalar(
        r#"
        SELECT m.role
        FROM library_memberships m
        INNER JOIN libraries l ON l.id = m.library_id
        WHERE m.library_id = $1 AND m.user_id = $2 AND l.deleted_at IS NULL
        "#,
    )
    .bind(library_id)
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await?;

    role_value
        .ok_or(AppError::Forbidden)?
        .parse::<Role>()
        .map_err(|_| AppError::Forbidden)
}

pub async fn ensure_library_access(
    state: &AppState,
    user: &AuthUser,
    library_id: &str,
) -> AppResult<Role> {
    let role = ensure_library_membership(state, user, library_id).await?;
    let enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM libraries WHERE id = $1 AND deleted_at IS NULL")
            .bind(library_id)
            .fetch_optional(&state.pool)
            .await?;

    match enabled {
        Some(true) => Ok(role),
        Some(false) => Err(AppError::LibraryDisabled(library_id.to_string())),
        None => Err(AppError::NotFound("library not found".to_string())),
    }
}

pub async fn ensure_library_manager(
    state: &AppState,
    user: &AuthUser,
    library_id: &str,
) -> AppResult<Role> {
    let role = ensure_library_membership(state, user, library_id).await?;
    if user.role.can_manage_server() || role.can_manage_library() {
        Ok(role)
    } else {
        Err(AppError::Forbidden)
    }
}

pub async fn ensure_library_asset_import_access(
    state: &AppState,
    user: &AuthUser,
    library_id: &str,
) -> AppResult<Role> {
    let role = ensure_library_access(state, user, library_id).await?;
    if role.can_import_assets() {
        Ok(role)
    } else {
        Err(AppError::Forbidden)
    }
}

pub async fn ensure_library_asset_mutation_access(
    state: &AppState,
    user: &AuthUser,
    library_id: &str,
    asset_ids: &[String],
) -> AppResult<Role> {
    let role = ensure_library_access(state, user, library_id).await?;
    if role.can_manage_all_assets() || asset_ids.is_empty() {
        return Ok(role);
    }

    if !role.can_manage_own_assets() {
        return Err(AppError::AssetMutationForbidden {
            denied_count: asset_ids.len(),
            total_count: asset_ids.len(),
        });
    }

    let (found_count, owned_count): (i64, i64) = sqlx::query_as(
        r#"
        SELECT COUNT(*), COUNT(*) FILTER (WHERE created_by_user_id = $3)
        FROM assets
        WHERE library_id = $1
          AND id = ANY($2)
        "#,
    )
    .bind(library_id)
    .bind(asset_ids)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;
    if found_count != asset_ids.len() as i64 {
        return Err(AppError::BadRequest(
            "one or more assets were not found".to_string(),
        ));
    }
    let denied_count = asset_ids.len().saturating_sub(owned_count as usize);
    if denied_count > 0 {
        return Err(AppError::AssetMutationForbidden {
            denied_count,
            total_count: asset_ids.len(),
        });
    }

    Ok(role)
}

pub async fn ensure_library_folder_create_access(
    state: &AppState,
    user: &AuthUser,
    library_id: &str,
    parent_id: Option<&str>,
) -> AppResult<Role> {
    let role = ensure_library_access(state, user, library_id).await?;
    if role.can_manage_all_folders() {
        return Ok(role);
    }
    if !role.can_manage_own_folders() {
        return Err(AppError::FolderMutationForbidden {
            denied_count: 1,
            total_count: 1,
        });
    }
    if let Some(parent_id) = parent_id {
        let parent_ids = [parent_id.to_string()];
        ensure_library_folder_mutation_access(state, user, library_id, &parent_ids).await?;
    }
    Ok(role)
}

pub async fn ensure_library_folder_mutation_access(
    state: &AppState,
    user: &AuthUser,
    library_id: &str,
    folder_ids: &[String],
) -> AppResult<Role> {
    let role = ensure_library_access(state, user, library_id).await?;
    if role.can_manage_all_folders() || folder_ids.is_empty() {
        return Ok(role);
    }
    if !role.can_manage_own_folders() {
        return Err(AppError::FolderMutationForbidden {
            denied_count: folder_ids.len().max(1),
            total_count: folder_ids.len().max(1),
        });
    }

    let (found_count, owned_count): (i64, i64) = sqlx::query_as(
        r#"
        SELECT COUNT(*), COUNT(*) FILTER (WHERE created_by_user_id = $3)
        FROM folders
        WHERE library_id = $1
          AND id = ANY($2)
        "#,
    )
    .bind(library_id)
    .bind(folder_ids)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;
    if found_count != folder_ids.len() as i64 {
        return Err(AppError::BadRequest(
            "one or more folders were not found".to_string(),
        ));
    }
    let denied_count = folder_ids.len().saturating_sub(owned_count as usize);
    if denied_count > 0 {
        return Err(AppError::FolderMutationForbidden {
            denied_count,
            total_count: folder_ids.len(),
        });
    }
    Ok(role)
}

pub async fn ensure_library_tag_create_access(
    state: &AppState,
    user: &AuthUser,
    library_id: &str,
    group_id: Option<&str>,
) -> AppResult<Role> {
    let role = ensure_library_access(state, user, library_id).await?;
    if role.can_manage_all_tags() {
        return Ok(role);
    }
    if !role.can_manage_own_tags() {
        return Err(AppError::TagMutationForbidden {
            denied_count: 1,
            total_count: 1,
        });
    }
    if let Some(group_id) = group_id {
        ensure_library_tag_group_mutation_access(state, user, library_id, &[group_id.to_string()])
            .await?;
    }
    Ok(role)
}

pub async fn ensure_library_tag_mutation_access(
    state: &AppState,
    user: &AuthUser,
    library_id: &str,
    tag_ids: &[String],
) -> AppResult<Role> {
    let role = ensure_library_access(state, user, library_id).await?;
    if role.can_manage_all_tags() || tag_ids.is_empty() {
        return Ok(role);
    }
    if !role.can_manage_own_tags() {
        return Err(AppError::TagMutationForbidden {
            denied_count: tag_ids.len().max(1),
            total_count: tag_ids.len().max(1),
        });
    }

    let (found_count, owned_count): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COUNT(*) FILTER (WHERE created_by_user_id = $3) FROM tags WHERE library_id = $1 AND id = ANY($2)",
    )
    .bind(library_id)
    .bind(tag_ids)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;
    if found_count != tag_ids.len() as i64 {
        return Err(AppError::BadRequest(
            "one or more tags were not found".to_string(),
        ));
    }
    let denied_count = tag_ids.len().saturating_sub(owned_count as usize);
    if denied_count > 0 {
        return Err(AppError::TagMutationForbidden {
            denied_count,
            total_count: tag_ids.len(),
        });
    }
    Ok(role)
}

pub async fn ensure_library_tag_group_create_access(
    state: &AppState,
    user: &AuthUser,
    library_id: &str,
) -> AppResult<Role> {
    let role = ensure_library_access(state, user, library_id).await?;
    if role.can_manage_own_tags() {
        Ok(role)
    } else {
        Err(AppError::TagGroupMutationForbidden {
            denied_count: 1,
            total_count: 1,
        })
    }
}

pub async fn ensure_library_tag_group_mutation_access(
    state: &AppState,
    user: &AuthUser,
    library_id: &str,
    group_ids: &[String],
) -> AppResult<Role> {
    let role = ensure_library_access(state, user, library_id).await?;
    if role.can_manage_all_tags() || group_ids.is_empty() {
        return Ok(role);
    }
    if !role.can_manage_own_tags() {
        return Err(AppError::TagGroupMutationForbidden {
            denied_count: group_ids.len().max(1),
            total_count: group_ids.len().max(1),
        });
    }

    let (found_count, owned_count): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COUNT(*) FILTER (WHERE created_by_user_id = $3) FROM tag_groups WHERE library_id = $1 AND id = ANY($2)",
    )
    .bind(library_id)
    .bind(group_ids)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;
    if found_count != group_ids.len() as i64 {
        return Err(AppError::BadRequest(
            "one or more tag groups were not found".to_string(),
        ));
    }
    let denied_count = group_ids.len().saturating_sub(owned_count as usize);
    if denied_count > 0 {
        return Err(AppError::TagGroupMutationForbidden {
            denied_count,
            total_count: group_ids.len(),
        });
    }
    Ok(role)
}
