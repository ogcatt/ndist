// src/backend/server_functions/groups.rs

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use super::super::db::get_db;

#[cfg(feature = "server")]
use entity::{group_members, groups, users};

#[cfg(feature = "server")]
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, PaginatorTrait};

#[cfg(feature = "server")]
use chrono::Utc;

#[cfg(feature = "server")]
use uuid::Uuid;

use super::auth::{check_admin_permission, get_current_user};

#[cfg(feature = "server")]
use super::basket::DbErrExt;

// ============================================================================
// Data Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub member_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroupMember {
    pub id: String,
    pub group_id: String,
    pub user_id: String,
    pub user_email: String,
    pub user_name: String,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserSearchResult {
    pub id: String,
    pub email: String,
    pub name: String,
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGroupResponse {
    pub success: bool,
    pub message: String,
    pub group_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetGroupRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetGroupResponse {
    pub success: bool,
    pub message: String,
    pub group: Option<Group>,
    pub members: Vec<GroupMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGroupRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGroupResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteGroupRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteGroupResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddGroupMemberRequest {
    pub group_id: String,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddGroupMemberResponse {
    pub success: bool,
    pub message: String,
    pub member_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveGroupMemberRequest {
    pub member_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveGroupMemberResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchUsersRequest {
    pub query: String,
    pub exclude_group_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchUsersResponse {
    pub success: bool,
    pub users: Vec<UserSearchResult>,
}

// ============================================================================
// Server Functions
// ============================================================================

#[server]
pub async fn admin_get_groups() -> Result<Vec<Group>, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Err(ServerFnError::new("Unauthorized"));
    }

    let db = get_db().await;

    let group_models: Vec<groups::Model> = groups::Entity::find().all(db).await.map_db_err()?;

    let mut groups_final = Vec::new();

    for group_model in group_models {
        let member_count = group_members::Entity::find()
            .filter(group_members::Column::GroupId.eq(&group_model.id))
            .count(db)
            .await
            .map_db_err()? as usize;

        groups_final.push(Group {
            id: group_model.id,
            name: group_model.name,
            description: group_model.description,
            created_at: group_model.created_at,
            updated_at: group_model.updated_at,
            member_count,
        });
    }

    Ok(groups_final)
}

#[server]
pub async fn admin_create_group(
    request: CreateGroupRequest,
) -> Result<CreateGroupResponse, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Ok(CreateGroupResponse {
            success: false,
            message: "Unauthorized".to_string(),
            group_id: None,
        });
    }

    let name = request.name.trim();

    if name.is_empty() {
        return Ok(CreateGroupResponse {
            success: false,
            message: "Group name is required".to_string(),
            group_id: None,
        });
    }

    if name.len() < 3 {
        return Ok(CreateGroupResponse {
            success: false,
            message: "Group name must be at least 3 characters".to_string(),
            group_id: None,
        });
    }

    if name.len() > 30 {
        return Ok(CreateGroupResponse {
            success: false,
            message: "Group name must be less than 30 characters".to_string(),
            group_id: None,
        });
    }

    let db = get_db().await;

    let existing_group = groups::Entity::find()
        .filter(groups::Column::Name.eq(name))
        .one(db)
        .await
        .map_db_err()?;

    if existing_group.is_some() {
        return Ok(CreateGroupResponse {
            success: false,
            message: "A group with this name already exists".to_string(),
            group_id: None,
        });
    }

    let group_id = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();

    let group = groups::ActiveModel {
        id: ActiveValue::Set(group_id.clone()),
        name: ActiveValue::Set(name.to_string()),
        description: ActiveValue::Set(request.description),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
    };

    groups::Entity::insert(group)
        .exec(db)
        .await
        .map_db_err()?;

    Ok(CreateGroupResponse {
        success: true,
        message: "Group created successfully".to_string(),
        group_id: Some(group_id),
    })
}

#[server]
pub async fn admin_get_group(
    request: GetGroupRequest,
) -> Result<GetGroupResponse, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Ok(GetGroupResponse {
            success: false,
            message: "Unauthorized".to_string(),
            group: None,
            members: Vec::new(),
        });
    }

    let db = get_db().await;

    let group_model = groups::Entity::find()
        .filter(groups::Column::Id.eq(&request.id))
        .one(db)
        .await
        .map_db_err()?;

    match group_model {
        Some(model) => {
            let member_models = group_members::Entity::find()
                .filter(group_members::Column::GroupId.eq(&request.id))
                .all(db)
                .await
                .map_db_err()?;

            let mut members = Vec::new();

            for member_model in member_models {
                let user_model = users::Entity::find()
                    .filter(users::Column::Id.eq(&member_model.user_id))
                    .one(db)
                    .await
                    .map_db_err()?;

                if let Some(user) = user_model {
                    members.push(GroupMember {
                        id: member_model.id,
                        group_id: member_model.group_id,
                        user_id: member_model.user_id,
                        user_email: user.email,
                        user_name: user.name,
                        created_at: member_model.created_at,
                    });
                }
            }

            let member_count = members.len();

            let group = Group {
                id: model.id,
                name: model.name,
                description: model.description,
                created_at: model.created_at,
                updated_at: model.updated_at,
                member_count,
            };

            Ok(GetGroupResponse {
                success: true,
                message: "Group found".to_string(),
                group: Some(group),
                members,
            })
        }
        None => Ok(GetGroupResponse {
            success: false,
            message: "Group not found".to_string(),
            group: None,
            members: Vec::new(),
        }),
    }
}

#[server]
pub async fn admin_update_group(
    request: UpdateGroupRequest,
) -> Result<UpdateGroupResponse, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Ok(UpdateGroupResponse {
            success: false,
            message: "Unauthorized".to_string(),
        });
    }

    let name = request.name.trim();

    if name.is_empty() {
        return Ok(UpdateGroupResponse {
            success: false,
            message: "Group name is required".to_string(),
        });
    }

    if name.len() < 3 {
        return Ok(UpdateGroupResponse {
            success: false,
            message: "Group name must be at least 3 characters".to_string(),
        });
    }

    if name.len() > 30 {
        return Ok(UpdateGroupResponse {
            success: false,
            message: "Group name must be less than 30 characters".to_string(),
        });
    }

    let db = get_db().await;

    let existing_group = groups::Entity::find()
        .filter(groups::Column::Id.eq(&request.id))
        .one(db)
        .await
        .map_db_err()?;

    let group_model = match existing_group {
        Some(model) => model,
        None => {
            return Ok(UpdateGroupResponse {
                success: false,
                message: "Group not found".to_string(),
            });
        }
    };

    let name_conflict = groups::Entity::find()
        .filter(groups::Column::Name.eq(name))
        .filter(groups::Column::Id.ne(&request.id))
        .one(db)
        .await
        .map_db_err()?;

    if name_conflict.is_some() {
        return Ok(UpdateGroupResponse {
            success: false,
            message: "A group with this name already exists".to_string(),
        });
    }

    let now = Utc::now().naive_utc();

    let updated_group = groups::ActiveModel {
        id: ActiveValue::Unchanged(group_model.id),
        name: ActiveValue::Set(name.to_string()),
        description: ActiveValue::Set(request.description),
        created_at: ActiveValue::Unchanged(group_model.created_at),
        updated_at: ActiveValue::Set(now),
    };

    groups::Entity::update(updated_group)
        .exec(db)
        .await
        .map_db_err()?;

    Ok(UpdateGroupResponse {
        success: true,
        message: "Group updated successfully".to_string(),
    })
}

#[server]
pub async fn admin_delete_group(
    request: DeleteGroupRequest,
) -> Result<DeleteGroupResponse, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Ok(DeleteGroupResponse {
            success: false,
            message: "Unauthorized".to_string(),
        });
    }

    let db = get_db().await;

    let res = groups::Entity::delete_by_id(request.id)
        .exec(db)
        .await
        .map_db_err()?;

    if res.rows_affected == 0 {
        Ok(DeleteGroupResponse {
            success: false,
            message: "Group not found".to_string(),
        })
    } else {
        Ok(DeleteGroupResponse {
            success: true,
            message: "Group deleted successfully".to_string(),
        })
    }
}

#[server]
pub async fn admin_add_group_member(
    request: AddGroupMemberRequest,
) -> Result<AddGroupMemberResponse, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Ok(AddGroupMemberResponse {
            success: false,
            message: "Unauthorized".to_string(),
            member_id: None,
        });
    }

    let db = get_db().await;

    let group_exists = groups::Entity::find()
        .filter(groups::Column::Id.eq(&request.group_id))
        .one(db)
        .await
        .map_db_err()?;

    if group_exists.is_none() {
        return Ok(AddGroupMemberResponse {
            success: false,
            message: "Group not found".to_string(),
            member_id: None,
        });
    }

    let user_exists = users::Entity::find()
        .filter(users::Column::Id.eq(&request.user_id))
        .one(db)
        .await
        .map_db_err()?;

    if user_exists.is_none() {
        return Ok(AddGroupMemberResponse {
            success: false,
            message: "User not found".to_string(),
            member_id: None,
        });
    }

    let existing_member = group_members::Entity::find()
        .filter(group_members::Column::GroupId.eq(&request.group_id))
        .filter(group_members::Column::UserId.eq(&request.user_id))
        .one(db)
        .await
        .map_db_err()?;

    if existing_member.is_some() {
        return Ok(AddGroupMemberResponse {
            success: false,
            message: "User is already a member of this group".to_string(),
            member_id: None,
        });
    }

    let member_id = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();

    let member = group_members::ActiveModel {
        id: ActiveValue::Set(member_id.clone()),
        group_id: ActiveValue::Set(request.group_id),
        user_id: ActiveValue::Set(request.user_id),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
    };

    group_members::Entity::insert(member)
        .exec(db)
        .await
        .map_db_err()?;

    Ok(AddGroupMemberResponse {
        success: true,
        message: "Member added successfully".to_string(),
        member_id: Some(member_id),
    })
}

#[server]
pub async fn admin_remove_group_member(
    request: RemoveGroupMemberRequest,
) -> Result<RemoveGroupMemberResponse, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Ok(RemoveGroupMemberResponse {
            success: false,
            message: "Unauthorized".to_string(),
        });
    }

    let db = get_db().await;

    let res = group_members::Entity::delete_by_id(request.member_id)
        .exec(db)
        .await
        .map_db_err()?;

    if res.rows_affected == 0 {
        Ok(RemoveGroupMemberResponse {
            success: false,
            message: "Member not found".to_string(),
        })
    } else {
        Ok(RemoveGroupMemberResponse {
            success: true,
            message: "Member removed successfully".to_string(),
        })
    }
}

#[server]
pub async fn admin_search_users(
    request: SearchUsersRequest,
) -> Result<SearchUsersResponse, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Ok(SearchUsersResponse {
            success: false,
            users: Vec::new(),
        });
    }

    let db = get_db().await;

    let query = request.query.trim().to_lowercase();

    if query.is_empty() {
        return Ok(SearchUsersResponse {
            success: true,
            users: Vec::new(),
        });
    }

    let all_users: Vec<users::Model> = users::Entity::find().all(db).await.map_db_err()?;

    let mut filtered_users: Vec<users::Model> = all_users
        .into_iter()
        .filter(|u| {
            u.email.to_lowercase().contains(&query) || u.name.to_lowercase().contains(&query)
        })
        .collect();

    if let Some(exclude_group_id) = request.exclude_group_id {
        let group_member_user_ids: Vec<String> = group_members::Entity::find()
            .filter(group_members::Column::GroupId.eq(&exclude_group_id))
            .all(db)
            .await
            .map_db_err()?
            .into_iter()
            .map(|m| m.user_id)
            .collect();

        filtered_users.retain(|u| !group_member_user_ids.contains(&u.id));
    }

    filtered_users.truncate(20);

    let users_result: Vec<UserSearchResult> = filtered_users
        .into_iter()
        .map(|u| UserSearchResult {
            id: u.id,
            email: u.email,
            name: u.name,
        })
        .collect();

    Ok(SearchUsersResponse {
        success: true,
        users: users_result,
    })
}
