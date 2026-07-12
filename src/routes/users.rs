use crate::{
    auth::{hash_password, AuthUser},
    error::{AppError, AppResult},
    models::{Role, UserLibraryMembershipRecord, UserRecord, UserWithMemberships},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use sqlx::{FromRow, Postgres, Transaction};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub(crate) mod guards;
mod requests;

use guards::{ensure_another_active_owner, ensure_unique_display_name};
use requests::{CreateUserRequest, UpdateUserRequest, UserLibraryMembershipInput};

pub async fn list_users(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<UserWithMemberships>>> {
    if !user.role.can_manage_server() {
        return Err(AppError::Forbidden);
    }

    let users = sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT
            u.id,
            u.email,
            u.display_name,
            u.global_role,
            u.is_active,
            u.last_login_at,
            u.last_seen_at,
            u.last_seen_library_id,
            l.display_name AS last_seen_library_name,
            u.created_at,
            u.updated_at
        FROM users u
        LEFT JOIN libraries l ON l.id = u.last_seen_library_id AND l.deleted_at IS NULL
        ORDER BY
            u.last_seen_at DESC NULLS LAST,
            u.display_name ASC,
            u.email ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(attach_library_memberships(&state, users).await?))
}

#[derive(FromRow)]
struct UserLibraryMembershipRow {
    user_id: Uuid,
    library_id: String,
    library_name: String,
    role: String,
}

async fn attach_library_memberships(
    state: &AppState,
    users: Vec<UserRecord>,
) -> AppResult<Vec<UserWithMemberships>> {
    let rows = sqlx::query_as::<_, UserLibraryMembershipRow>(
        r#"
        SELECT
            m.user_id,
            m.library_id,
            l.display_name AS library_name,
            m.role
        FROM library_memberships m
        INNER JOIN libraries l ON l.id = m.library_id
        WHERE l.deleted_at IS NULL
        ORDER BY l.display_name ASC, m.library_id ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    let mut memberships_by_user: HashMap<Uuid, Vec<UserLibraryMembershipRecord>> = HashMap::new();
    for row in rows {
        memberships_by_user
            .entry(row.user_id)
            .or_default()
            .push(UserLibraryMembershipRecord {
                library_id: row.library_id,
                library_name: row.library_name,
                role: row.role,
            });
    }

    Ok(users
        .into_iter()
        .map(|user| {
            let library_memberships = memberships_by_user.remove(&user.id).unwrap_or_default();
            UserWithMemberships {
                user,
                library_memberships,
            }
        })
        .collect())
}

pub async fn create_user(
    State(state): State<AppState>,
    actor: AuthUser,
    Json(request): Json<CreateUserRequest>,
) -> AppResult<Json<UserWithMemberships>> {
    if !actor.role.can_manage_server() {
        return Err(AppError::Forbidden);
    }

    let role = request.role.unwrap_or(Role::Viewer);
    if role == Role::Owner && actor.role != Role::Owner {
        return Err(AppError::Forbidden);
    }

    let email = request.email.trim().to_ascii_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(AppError::BadRequest("valid email is required".to_string()));
    }
    let existing_user: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&state.pool)
        .await?;
    if existing_user.is_some() {
        return Err(AppError::Conflict("email already exists".to_string()));
    }
    if request.password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".to_string(),
        ));
    }

    let display_name = request
        .display_name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| email.clone())
        .trim()
        .to_string();
    ensure_unique_display_name(&state, &display_name, None).await?;

    let user_id = Uuid::new_v4();
    let password_hash = hash_password(&request.password)?;
    let mut tx = state.pool.begin().await?;

    let user = sqlx::query_as::<_, UserRecord>(
        r#"
        INSERT INTO users (id, email, display_name, password_hash, global_role)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, email, display_name, global_role, is_active, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(email)
    .bind(display_name)
    .bind(password_hash)
    .bind(role.as_str())
    .fetch_one(&mut *tx)
    .await?;

    if let Some(memberships) = request.library_memberships.as_deref() {
        replace_library_memberships(&mut tx, actor.role, user_id, memberships).await?;
    }

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, actor_user_id, action, target_type, target_id)
        VALUES ($1, $2, 'user.created', 'user', $3)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(actor.id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let mut users = attach_library_memberships(&state, vec![user]).await?;
    Ok(Json(users.pop().expect("created user response is present")))
}

pub async fn update_user(
    State(state): State<AppState>,
    actor: AuthUser,
    Path(user_id): Path<Uuid>,
    Json(request): Json<UpdateUserRequest>,
) -> AppResult<Json<UserWithMemberships>> {
    if !actor.role.can_manage_server() {
        return Err(AppError::Forbidden);
    }

    let target = sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, email, display_name, global_role, is_active, created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("user not found".to_string()))?;

    let current_role = target
        .global_role
        .parse::<Role>()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("stored user role is invalid")))?;

    if actor.role != Role::Owner
        && (current_role == Role::Owner || request.role == Some(Role::Owner))
    {
        return Err(AppError::Forbidden);
    }

    let next_role = request.role.unwrap_or(current_role);
    if next_role == Role::Owner && actor.role != Role::Owner {
        return Err(AppError::Forbidden);
    }

    let next_is_active = request.is_active.unwrap_or(target.is_active);
    if current_role == Role::Owner && (next_role != Role::Owner || !next_is_active) {
        ensure_another_active_owner(&state, user_id).await?;
    }

    if let Some(password) = &request.password {
        if !password.is_empty() && password.len() < 8 {
            return Err(AppError::BadRequest(
                "password must be at least 8 characters".to_string(),
            ));
        }
    }

    let next_display_name = request
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&target.display_name)
        .to_string();
    ensure_unique_display_name(&state, &next_display_name, Some(user_id)).await?;

    let password_hash = match request.password.as_deref().map(str::trim) {
        Some(password) if !password.is_empty() => Some(hash_password(password)?),
        _ => None,
    };
    let password_changed = password_hash.is_some();

    let mut tx = state.pool.begin().await?;

    let user = sqlx::query_as::<_, UserRecord>(
        r#"
        UPDATE users
        SET
            display_name = $2,
            global_role = $3,
            is_active = $4,
            password_hash = COALESCE($5, password_hash),
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, email, display_name, global_role, is_active, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(&next_display_name)
    .bind(next_role.as_str())
    .bind(next_is_active)
    .bind(password_hash.as_deref())
    .fetch_one(&mut *tx)
    .await?;

    if let Some(memberships) = request.library_memberships.as_deref() {
        replace_library_memberships(&mut tx, actor.role, user_id, memberships).await?;
    }

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, actor_user_id, action, target_type, target_id, details)
        VALUES ($1, $2, 'user.updated', 'user', $3, $4::jsonb)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(actor.id)
    .bind(user_id)
    .bind(serde_json::json!({
        "role": next_role.as_str(),
        "isActive": next_is_active,
        "passwordChanged": password_changed,
        "libraryMembershipsChanged": request.library_memberships.is_some()
    }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let mut users = attach_library_memberships(&state, vec![user]).await?;
    Ok(Json(users.pop().expect("updated user response is present")))
}

async fn replace_library_memberships(
    tx: &mut Transaction<'_, Postgres>,
    actor_role: Role,
    user_id: Uuid,
    memberships: &[UserLibraryMembershipInput],
) -> AppResult<()> {
    let mut desired_roles = HashMap::new();
    for membership in memberships {
        let library_id = membership.library_id.trim();
        if library_id.is_empty() {
            return Err(AppError::BadRequest("library id is required".to_string()));
        }
        if matches!(membership.role, Role::Owner | Role::Admin) && actor_role != Role::Owner {
            return Err(AppError::Forbidden);
        }
        if desired_roles
            .insert(library_id.to_string(), membership.role)
            .is_some()
        {
            return Err(AppError::BadRequest(
                "library membership is duplicated".to_string(),
            ));
        }
    }

    let current_rows = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT m.library_id, m.role
        FROM library_memberships m
        INNER JOIN libraries l ON l.id = m.library_id
        WHERE m.user_id = $1 AND l.deleted_at IS NULL
        "#,
    )
    .bind(user_id)
    .fetch_all(&mut **tx)
    .await?;

    let mut library_ids = desired_roles.keys().cloned().collect::<HashSet<_>>();
    library_ids.extend(
        current_rows
            .iter()
            .map(|(library_id, _)| library_id.clone()),
    );
    let mut library_ids = library_ids.into_iter().collect::<Vec<_>>();
    library_ids.sort();
    for library_id in &library_ids {
        let exists: Option<String> = sqlx::query_scalar(
            "SELECT id FROM libraries WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(library_id)
        .fetch_optional(&mut **tx)
        .await?;
        if exists.is_none() {
            return Err(AppError::NotFound(format!(
                "library {library_id} not found"
            )));
        }
    }

    for (library_id, current_role) in &current_rows {
        let next_role = desired_roles.get(library_id).copied();
        if is_library_manager_role(current_role) && !next_role.is_some_and(Role::can_manage_library)
        {
            let other_manager_count: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*)
                FROM library_memberships
                WHERE library_id = $1
                  AND user_id <> $2
                  AND role IN ('owner', 'admin', 'library_manager')
                "#,
            )
            .bind(library_id)
            .bind(user_id)
            .fetch_one(&mut **tx)
            .await?;
            if other_manager_count == 0 {
                return Err(AppError::Conflict(format!(
                    "library {library_id} requires at least one manager"
                )));
            }
        }
    }

    let desired_ids = desired_roles.keys().cloned().collect::<HashSet<_>>();
    for (library_id, _) in current_rows {
        if !desired_ids.contains(&library_id) {
            sqlx::query("DELETE FROM library_memberships WHERE library_id = $1 AND user_id = $2")
                .bind(&library_id)
                .bind(user_id)
                .execute(&mut **tx)
                .await?;
        }
    }

    for (library_id, role) in desired_roles {
        sqlx::query(
            r#"
            INSERT INTO library_memberships (library_id, user_id, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (library_id, user_id)
            DO UPDATE SET role = EXCLUDED.role, updated_at = NOW()
            "#,
        )
        .bind(library_id)
        .bind(user_id)
        .bind(role.as_str())
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

fn is_library_manager_role(role: &str) -> bool {
    matches!(role, "owner" | "admin" | "library_manager")
}
