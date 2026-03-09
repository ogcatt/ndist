#![allow(non_snake_case)]

use crate::backend::server_functions::{
    AddGroupMemberRequest, CreateGroupRequest, DeleteGroupRequest, GetGroupRequest,
    RemoveGroupMemberRequest, SearchUsersRequest, UpdateGroupRequest, admin_add_group_member,
    admin_create_group, admin_delete_group, admin_get_group, admin_remove_group_member,
    admin_search_users, admin_update_group, GroupMember, UserSearchResult,
    admin_create_invite_code, admin_get_group_codes, admin_revoke_invite_code,
    admin_delete_invite_code, admin_create_api_key, admin_get_api_keys,
    admin_revoke_api_key, admin_delete_api_key, InviteCodeInfo, ApiKeyInfo, CreateApiKeyResponse,
};
use crate::components::*;
use dioxus::prelude::*;

#[derive(PartialEq, Props, Clone)]
pub struct AdminGroupProps {
    pub id: Option<Signal<String>>,
}

#[component]
pub fn AdminGroup(props: AdminGroupProps) -> Element {
    let is_edit_mode = props.id.is_some();
    let mut group_id = use_signal(|| String::new());
    let props_id = props.id;

    use_effect(move || {
        if let Some(id) = props_id {
            group_id.set(id());
        }
    });

    let mut name = use_signal(|| String::new());
    let mut description = use_signal(|| String::new());
    let mut members = use_signal(|| Vec::<GroupMember>::new());
    let mut pending_members = use_signal(|| Vec::<UserSearchResult>::new());

    let mut saving = use_signal(|| false);
    let mut deleting = use_signal(|| false);
    let mut is_deleted = use_signal(|| false);
    let mut loaded = use_signal(|| false);

    let mut notification_message = use_signal(|| String::new());
    let mut notification_type = use_signal(|| String::new());
    let mut show_notification = use_signal(|| false);

    let mut user_search_query = use_signal(|| String::new());
    let mut user_search_results = use_signal(|| Vec::<UserSearchResult>::new());
    let mut searching_users = use_signal(|| false);

    // Invite codes state
    let mut invite_codes = use_signal(|| Vec::<InviteCodeInfo>::new());
    let mut codes_loaded = use_signal(|| false);
    let mut generating_code = use_signal(|| false);
    let mut expanded_code_id = use_signal(|| Option::<String>::None);

    // API keys state
    let mut api_keys_list = use_signal(|| Vec::<ApiKeyInfo>::new());
    let mut api_keys_loaded = use_signal(|| false);
    let mut new_api_key_name = use_signal(|| String::new());
    let mut creating_api_key = use_signal(|| false);
    let mut newly_created_key = use_signal(|| Option::<String>::None);

    let group_resource = use_resource(move || async move {
        if is_edit_mode {
            // Wait for group_id to be populated by use_effect
            if group_id().is_empty() {
                return Err(ServerFnError::new("Loading..."));
            }
            admin_get_group(GetGroupRequest { id: group_id() }).await
        } else {
            Err(ServerFnError::new("Not in edit mode"))
        }
    });

    use_effect(move || {
        if !is_edit_mode {
            loaded.set(false);
            return;
        }

        if let Some(res) = group_resource() {
            match res {
                Ok(resp) => {
                    if resp.success {
                        if let Some(g) = resp.group.clone() {
                            if !*loaded.peek() {
                                name.set(g.name);
                                description.set(g.description.unwrap_or_default());
                                members.set(resp.members);
                                loaded.set(true);
                            }
                            // Load invite codes and API keys once
                            if !*codes_loaded.peek() {
                                let gid = g.id.clone();
                                let gid2 = g.id.clone();
                                spawn(async move {
                                    if let Ok(codes) = admin_get_group_codes(gid).await {
                                        invite_codes.set(codes);
                                    }
                                });
                                spawn(async move {
                                    if let Ok(keys) = admin_get_api_keys(gid2).await {
                                        api_keys_list.set(keys);
                                    }
                                });
                                codes_loaded.set(true);
                            }
                        } else {
                            notification_message.set("Group not found".to_string());
                            notification_type.set("error".to_string());
                            show_notification.set(true);
                        }
                    } else {
                        notification_message.set(resp.message.clone());
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                    }
                }
                Err(e) => {
                    // Don't show notification for loading state (group_id not yet populated)
                    if e.to_string().contains("Loading...") || e.to_string().contains("Not in edit mode") {
                        return;
                    }
                    notification_message.set(format!("Error loading group: {}", e));
                    notification_type.set("error".to_string());
                    show_notification.set(true);
                }
            }
        }
    });

    let handle_save_group = move |_: Event<MouseData>| {
        let group_id = group_id();
        let name = name();
        let description = description();
        let pending_members_list = pending_members();
        let is_edit_mode = props.id.is_some();

        spawn(async move {
            saving.set(true);

            if name.trim().is_empty() {
                notification_message.set("Group name is required".to_string());
                notification_type.set("error".to_string());
                show_notification.set(true);
                saving.set(false);
                return;
            }

            if name.trim().len() < 3 {
                notification_message.set("Group name must be at least 3 characters".to_string());
                notification_type.set("error".to_string());
                show_notification.set(true);
                saving.set(false);
                return;
            }

            if name.trim().len() > 30 {
                notification_message
                    .set("Group name must be less than 30 characters".to_string());
                notification_type.set("error".to_string());
                show_notification.set(true);
                saving.set(false);
                return;
            }

            let (save_success, target_group_id) = if is_edit_mode {
                match admin_update_group(UpdateGroupRequest {
                    id: group_id.clone(),
                    name: name.trim().to_string(),
                    description: if description.trim().is_empty() {
                        None
                    } else {
                        Some(description.trim().to_string())
                    },
                })
                .await
                {
                    Ok(resp) => (Ok(resp.success), group_id),
                    Err(e) => (Err(e), group_id),
                }
            } else {
                match admin_create_group(CreateGroupRequest {
                    name: name.trim().to_string(),
                    description: if description.trim().is_empty() {
                        None
                    } else {
                        Some(description.trim().to_string())
                    },
                })
                .await
                {
                    Ok(resp) => (Ok(resp.success), resp.group_id.unwrap_or_default()),
                    Err(e) => (Err(e), String::new()),
                }
            };

            match save_success {
                Ok(success) => {
                    if success {

                        let mut all_members_added = true;
                        for pending_user in &pending_members_list {
                            match admin_add_group_member(AddGroupMemberRequest {
                                group_id: target_group_id.clone(),
                                user_id: pending_user.id.clone(),
                            })
                            .await
                            {
                                Ok(resp) => {
                                    if !resp.success {
                                        all_members_added = false;
                                    }
                                }
                                Err(_) => {
                                    all_members_added = false;
                                }
                            }
                        }

                        pending_members.set(Vec::new());

                        notification_message.set(if is_edit_mode {
                            if all_members_added || pending_members_list.is_empty() {
                                "Group updated successfully!".to_string()
                            } else {
                                "Group updated, but some members could not be added".to_string()
                            }
                        } else {
                            if all_members_added || pending_members_list.is_empty() {
                                "Group created successfully!".to_string()
                            } else {
                                "Group created, but some members could not be added".to_string()
                            }
                        });
                        notification_type.set("success".to_string());
                        show_notification.set(true);

                        if is_edit_mode {
                            if let Ok(resp) = admin_get_group(GetGroupRequest {
                                id: target_group_id,
                            })
                            .await
                            {
                                if resp.success {
                                    members.set(resp.members);
                                }
                            }
                        }
                    } else {
                        notification_message.set("Operation failed".to_string());
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                    }
                }
                Err(e) => {
                    notification_message.set(format!("Error saving group: {}", e));
                    notification_type.set("error".to_string());
                    show_notification.set(true);
                }
            }

            saving.set(false);
        });
    };

    let handle_delete_group = move |_: Event<MouseData>| {
        let group_id = group_id();
        spawn(async move {
            deleting.set(true);

            match admin_delete_group(DeleteGroupRequest { id: group_id }).await {
                Ok(response) => {
                    if response.success {
                        notification_message.set("Group deleted successfully!".to_string());
                        notification_type.set("success".to_string());
                        show_notification.set(true);
                        is_deleted.set(true);
                    } else {
                        notification_message.set(response.message);
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                    }
                }
                Err(e) => {
                    notification_message.set(format!("Error deleting group: {}", e));
                    notification_type.set("error".to_string());
                    show_notification.set(true);
                }
            }

            deleting.set(false);
        });
    };

    let handle_search_users = move |_| {
        let query = user_search_query();
        let exclude_group_id = if is_edit_mode {
            Some(group_id())
        } else {
            None
        };

        spawn(async move {
            if query.trim().is_empty() {
                user_search_results.set(Vec::new());
                return;
            }

            searching_users.set(true);

            match admin_search_users(SearchUsersRequest {
                query,
                exclude_group_id,
            })
            .await
            {
                Ok(response) => {
                    if response.success {
                        let pending_ids: Vec<String> =
                            pending_members().iter().map(|u| u.id.clone()).collect();
                        let filtered_results: Vec<UserSearchResult> = response
                            .users
                            .into_iter()
                            .filter(|u| !pending_ids.contains(&u.id))
                            .collect();
                        user_search_results.set(filtered_results);
                    } else {
                        user_search_results.set(Vec::new());
                    }
                }
                Err(_) => {
                    user_search_results.set(Vec::new());
                }
            }

            searching_users.set(false);
        });
    };

    let mut handle_add_user_to_pending = move |user: UserSearchResult| {
        let mut pending = pending_members();
        if !pending.iter().any(|u| u.id == user.id) {
            pending.push(user);
            pending_members.set(pending);
        }
        user_search_query.set(String::new());
        user_search_results.set(Vec::new());
    };

    let mut handle_remove_pending_member = move |user_id: String| {
        let mut pending = pending_members();
        pending.retain(|u| u.id != user_id);
        pending_members.set(pending);
    };

    let mut handle_remove_existing_member = move |member_id: String| {
        spawn(async move {
            match admin_remove_group_member(RemoveGroupMemberRequest { member_id }).await {
                Ok(response) => {
                    if response.success {
                        if let Ok(resp) = admin_get_group(GetGroupRequest { id: group_id() }).await
                        {
                            if resp.success {
                                members.set(resp.members);
                            }
                        }
                        notification_message.set("Member removed successfully".to_string());
                        notification_type.set("success".to_string());
                        show_notification.set(true);
                    } else {
                        notification_message.set(response.message);
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                    }
                }
                Err(e) => {
                    notification_message.set(format!("Error removing member: {}", e));
                    notification_type.set("error".to_string());
                    show_notification.set(true);
                }
            }
        });
    };

    if is_edit_mode && !loaded() {
        let res = group_resource();
        let res_ref = res.as_ref();

        let is_loading_err = res_ref.is_some_and(|r| {
            r.as_ref().is_err_and(|e| e.to_string().contains("Loading..."))
        });

        let is_still_loading = res.is_none() || group_id().is_empty() || is_loading_err;

        let has_real_error = !is_still_loading
            && (res_ref.is_some_and(|r| r.is_err())
                || res_ref.is_some_and(|r| r.as_ref().is_ok_and(|resp| !resp.success)));

        if is_still_loading {
            return rsx! { div { class: "p-6 text-sm text-gray-400", "Loading..." } };
        }

        if has_real_error {
            return rsx! {
                div { class: "p-6 text-sm text-red-500", "Error loading group." }
            };
        }
    }

    rsx! {
        if show_notification() {
            div {
                class: format!(
                    "fixed top-4 right-4 z-50 p-4 rounded-md shadow-lg text-white {}",
                    if notification_type() == "success" { "bg-green-500" } else { "bg-red-500" }
                ),
                div {
                    class: "flex justify-between items-center",
                    span { "{notification_message()}" }
                    button {
                        class: "ml-4 text-white hover:text-gray-200",
                        onclick: move |_| show_notification.set(false),
                        "×"
                    }
                }
            }
        }

        if is_deleted() {
            div { class: "text-green-500 text-lg font-medium", "Group deleted successfully." }
        } else {
            div {
                class: "border rounded-md border-gray-200 w-full mb-4 flex justify-between items-center p-2 pl-4",
                div {
                    class: "text-lg font-medium",
                    if is_edit_mode { "Edit Group" } else { "Create New Group" }
                }
                div {
                    class: "flex items-center gap-2",
                    if is_edit_mode {
                        button {
                            class: format!(
                                "text-red-500 hover:text-red-700 {}",
                                if deleting() { "cursor-not-allowed opacity-50" } else { "" }
                            ),
                            disabled: deleting(),
                            onclick: handle_delete_group,
                            svg {
                                xmlns: "http://www.w3.org/2000/svg",
                                width: "20",
                                height: "20",
                                fill: "none",
                                path {
                                    stroke: "currentColor",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "1.5",
                                    d: "m12.283 7.5-.288 7.5m-3.99 0-.288-7.5m8.306-2.675c.285.043.569.09.852.138m-.852-.137-.89 11.568a1.875 1.875 0 0 1-1.87 1.73H6.737a1.875 1.875 0 0 1-1.87-1.73l-.89-11.569m12.046 0a40.08 40.08 0 0 0-2.898-.33m-10 .467c.283-.049.567-.095.852-.137m0 0a40.091 40.09 0 0 1 2.898-.33m6.25 0V3.73c0-.984-.758-1.804-1.742-1.834a43.3 43.3 0 0 0-2.766 0c-.984.03-1.742.851-1.742 1.834v.763m6.25 0c-2.08-.16-4.17-.16-6.25 0"
                                }
                            }
                        }
                    }
                    button {
                        class: format!(
                            "text-sm px-3 py-2 text-white rounded transition-colors {}",
                            if saving() {
                                "bg-gray-500 cursor-not-allowed"
                            } else {
                                "bg-gray-900 hover:bg-gray-800"
                            }
                        ),
                        disabled: saving(),
                        onclick: handle_save_group,
                        if saving() {
                            if is_edit_mode { "Updating..." } else { "Creating..." }
                        } else {
                            if is_edit_mode { "Update" } else { "Create" }
                        }
                    }
                }
            }

            div {
                class: "flex flex-col md:flex-row w-full gap-2",
                div {
                    class: "flex w-full flex-col gap-2",
                    div {
                        class: "flex-grow bg-white border rounded-md border-gray-200 p-4 min-h-36",
                        h2 { class: "text-lg font-medium mb-4", "Basic Information" }
                        div {
                            class: "flex flex-col gap-4 w-full",
                            div { class: "w-full",
                                CTextBox {
                                    label: "Group Name",
                                    value: "{name}",
                                    placeholder: "Engineering Team",
                                    optional: false,
                                    oninput: move |event: FormEvent| name.set(event.value())
                                }
                            }
                            div { class: "w-full",
                                label {
                                    class: "text-xs font-medium text-gray-700 pb-1",
                                    "Description"
                                }
                                textarea {
                                    class: "w-full px-2.5 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 text-sm",
                                    rows: "3",
                                    placeholder: "Optional description of the group...",
                                    value: "{description}",
                                    oninput: move |event: FormEvent| description.set(event.value())
                                }
                            }
                        }
                    }

                    div {
                        class: "flex-grow bg-white border rounded-md border-gray-200 p-4 min-h-36",
                        h2 { class: "text-lg font-medium mb-4", "Add Members" }
                        div { class: "flex flex-col gap-4",
                            div {
                                class: "relative",
                                CTextBox {
                                    label: "Search Users",
                                    value: "{user_search_query}",
                                    placeholder: "Search by email or name...",
                                    optional: true,
                                    oninput: move |event: FormEvent| {
                                        user_search_query.set(event.value());
                                        handle_search_users(());
                                    }
                                }

                                if !user_search_results().is_empty() {
                                    div {
                                        class: "absolute z-10 mt-1 w-full bg-white border border-gray-300 rounded-md shadow-lg max-h-60 overflow-y-auto",
                                        for user in user_search_results() {
                                            button {
                                                key: "{user.id}",
                                                class: "w-full px-3 py-2 text-left hover:bg-gray-100 border-b border-gray-200 last:border-b-0",
                                                onclick: {
                                                    let user_clone = user.clone();
                                                    move |_| handle_add_user_to_pending(user_clone.clone())
                                                },
                                                div {
                                                    class: "text-sm font-medium text-gray-900",
                                                    "{user.email}"
                                                    if user.email != user.name {
                                                        span {
                                                            class: "text-gray-500 ml-1",
                                                            "({user.name})"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if !pending_members().is_empty() {
                                div {
                                    class: "flex flex-col gap-2",
                                    label { class: "text-sm font-medium text-gray-700", "Pending Members" }
                                    div {
                                        class: "flex flex-wrap gap-2",
                                        for user in pending_members() {
                                            div {
                                                key: "{user.id}",
                                                class: "inline-flex items-center px-2 py-1 rounded-md bg-blue-100 text-blue-800 text-sm",
                                                span {
                                                    "{user.email}"
                                                    if user.email != user.name {
                                                        span {
                                                            class: "text-blue-600 ml-1",
                                                            "({user.name})"
                                                        }
                                                    }
                                                }
                                                button {
                                                    class: "ml-2 text-blue-600 hover:text-blue-800",
                                                    onclick: {
                                                        let user_id = user.id.clone();
                                                        move |_| handle_remove_pending_member(user_id.clone())
                                                    },
                                                    "×"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if is_edit_mode {
                    div { class: "md:w-[38%] w-full min-w-0",
                        div {
                            class: "bg-white border rounded-md border-gray-200 p-4 min-h-36 mb-2",
                            h2 { class: "text-lg font-medium mb-4", "Current Members" }

                            if members().is_empty() {
                                div {
                                    class: "text-gray-500 text-sm text-center py-4",
                                    "No members in this group yet"
                                }
                            } else {
                                div {
                                    class: "flex flex-col gap-2",
                                    for member in members() {
                                        div {
                                            key: "{member.id}",
                                            class: "flex justify-between items-center px-3 py-2 border border-gray-200 rounded-md hover:bg-gray-50",
                                            div {
                                                class: "flex flex-col",
                                                div {
                                                    class: "text-sm font-medium text-gray-900",
                                                    "{member.user_email}"
                                                }
                                                if member.user_email != member.user_name {
                                                    div {
                                                        class: "text-xs text-gray-500",
                                                        "{member.user_name}"
                                                    }
                                                }
                                            }
                                            button {
                                                class: "text-red-500 hover:text-red-700",
                                                onclick: {
                                                    let member_id = member.id.clone();
                                                    move |_| handle_remove_existing_member(member_id.clone())
                                                },
                                                svg {
                                                    xmlns: "http://www.w3.org/2000/svg",
                                                    width: "16",
                                                    height: "16",
                                                    fill: "none",
                                                    view_box: "0 0 24 24",
                                                    path {
                                                        stroke: "currentColor",
                                                        stroke_linecap: "round",
                                                        stroke_linejoin: "round",
                                                        stroke_width: "2",
                                                        d: "M6 18L18 6M6 6l12 12"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Invite Codes & API Keys sections (edit mode only)
            if !group_id().is_empty() {
                // Invite Codes Section
                div {
                    class: "bg-white border rounded-md border-gray-200 p-4 mt-2",
                    div {
                        class: "flex justify-between items-center mb-4",
                        h2 { class: "text-lg font-medium", "Invite Codes" }
                        button {
                            class: format!(
                                "text-sm px-3 py-1.5 text-white rounded transition-colors {}",
                                if generating_code() { "bg-gray-400 cursor-not-allowed" } else { "bg-gray-900 hover:bg-gray-800" }
                            ),
                            disabled: generating_code(),
                            onclick: move |_| {
                                let gid = group_id();
                                spawn(async move {
                                    generating_code.set(true);
                                    match admin_create_invite_code(gid).await {
                                        Ok(info) => {
                                            let mut codes = invite_codes();
                                            codes.insert(0, info);
                                            invite_codes.set(codes);
                                        }
                                        Err(e) => {
                                            notification_message.set(format!("Error generating code: {}", e));
                                            notification_type.set("error".to_string());
                                            show_notification.set(true);
                                        }
                                    }
                                    generating_code.set(false);
                                });
                            },
                            if generating_code() { "Generating..." } else { "Generate Code" }
                        }
                    }

                    if invite_codes().is_empty() {
                        div { class: "text-gray-500 text-sm text-center py-4", "No invite codes yet." }
                    } else {
                        div { class: "flex flex-col gap-2 max-h-64 overflow-y-auto pr-1",
                            for code_info in invite_codes() {
                                {
                                    let is_expanded = expanded_code_id().as_deref() == Some(&code_info.id);
                                    let cid_expand = code_info.id.clone();
                                    let created_fmt = code_info.created_at.format("%Y-%m-%d %H:%M").to_string();
                                    let used_fmt = code_info.used_at.map(|t| t.format("%Y-%m-%d %H:%M").to_string());
                                    rsx! {
                                        div {
                                            key: "{code_info.id}",
                                            class: "border border-gray-200 rounded-md text-sm overflow-hidden",
                                            // Main row
                                            div {
                                                class: "flex justify-between items-center px-3 py-2 cursor-pointer hover:bg-gray-50",
                                                onclick: move |_| {
                                                    if is_expanded {
                                                        expanded_code_id.set(None);
                                                    } else {
                                                        expanded_code_id.set(Some(cid_expand.clone()));
                                                    }
                                                },
                                                div { class: "flex flex-col gap-0.5",
                                                    div {
                                                        class: "font-mono font-semibold tracking-widest text-gray-900",
                                                        "{code_info.code}"
                                                    }
                                                    div { class: "flex gap-2 text-xs",
                                                        if code_info.is_revoked {
                                                            span { class: "text-red-500 font-medium", "Revoked" }
                                                        } else if code_info.is_used {
                                                            span { class: "text-gray-500",
                                                                "Used"
                                                                if let Some(ref email) = code_info.used_by_email {
                                                                    span { " by {email}" }
                                                                }
                                                            }
                                                        } else {
                                                            span { class: "text-green-600 font-medium", "Available" }
                                                        }
                                                        if code_info.is_api_generated {
                                                            span { class: "text-blue-500", "(API)" }
                                                        }
                                                    }
                                                }
                                                div { class: "flex gap-2 items-center",
                                                    if !code_info.is_revoked && !code_info.is_used {
                                                        button {
                                                            class: "text-xs text-yellow-600 hover:text-yellow-800 px-2 py-1 border border-yellow-300 rounded",
                                                            onclick: {
                                                                let cid = code_info.id.clone();
                                                                move |e| {
                                                                    e.stop_propagation();
                                                                    let cid = cid.clone();
                                                                    spawn(async move {
                                                                        if let Ok(true) = admin_revoke_invite_code(cid).await {
                                                                            if let Ok(updated) = admin_get_group_codes(group_id()).await {
                                                                                invite_codes.set(updated);
                                                                            }
                                                                        }
                                                                    });
                                                                }
                                                            },
                                                            "Revoke"
                                                        }
                                                    }
                                                    button {
                                                        class: "text-xs text-red-500 hover:text-red-700 px-2 py-1 border border-red-200 rounded",
                                                        onclick: {
                                                            let cid = code_info.id.clone();
                                                            move |e| {
                                                                e.stop_propagation();
                                                                let cid = cid.clone();
                                                                spawn(async move {
                                                                    if let Ok(true) = admin_delete_invite_code(cid).await {
                                                                        if let Ok(updated) = admin_get_group_codes(group_id()).await {
                                                                            invite_codes.set(updated);
                                                                        }
                                                                    }
                                                                });
                                                            }
                                                        },
                                                        "Delete"
                                                    }
                                                    span { class: "text-gray-400 text-xs ml-1", if is_expanded { "▲" } else { "▼" } }
                                                }
                                            }
                                            // Expanded details
                                            if is_expanded {
                                                div { class: "px-3 py-2 bg-gray-50 border-t border-gray-200 text-xs text-gray-600 flex flex-col gap-1",
                                                    div { "Created: {created_fmt}" }
                                                    div {
                                                        if let Some(ref used_str) = used_fmt {
                                                            "Used at: {used_str}"
                                                        } else {
                                                            "Not used yet"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // API Keys Section
                div {
                    class: "bg-white border rounded-md border-gray-200 p-4 mt-2",
                    div { class: "mb-4",
                        h2 { class: "text-lg font-medium", "API Keys" }
                        div { class: "mt-1 p-2 bg-gray-50 border border-gray-200 rounded text-xs text-gray-600 font-mono",
                            div { class: "font-semibold text-gray-700 mb-1 font-sans", "API Endpoint" }
                            div { "POST /novapi/v1/groups/invite/generate" }
                            div { class: "mt-1 text-gray-500", "Body: {{\"api_key\": \"sk_...\", \"group_id\": \"...\"}}" }
                        }
                    }

                    // New key creation form
                    div { class: "flex gap-2 mb-4",
                        input {
                            class: "flex-1 px-2.5 py-2 border border-gray-300 rounded-md text-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500",
                            placeholder: "Key name (e.g. My Integration)",
                            value: "{new_api_key_name}",
                            oninput: move |e| new_api_key_name.set(e.value()),
                        }
                        button {
                            class: format!(
                                "text-sm px-3 py-1.5 text-white rounded transition-colors whitespace-nowrap {}",
                                if creating_api_key() { "bg-gray-400 cursor-not-allowed" } else { "bg-gray-900 hover:bg-gray-800" }
                            ),
                            disabled: creating_api_key(),
                            onclick: move |_| {
                                let gid = group_id();
                                let name_val = new_api_key_name();
                                if name_val.trim().is_empty() {
                                    return;
                                }
                                spawn(async move {
                                    creating_api_key.set(true);
                                    match admin_create_api_key(gid.clone(), name_val).await {
                                        Ok(resp) => {
                                            if resp.success {
                                                newly_created_key.set(resp.plaintext_key);
                                                new_api_key_name.set(String::new());
                                                if let Ok(keys) = admin_get_api_keys(gid).await {
                                                    api_keys_list.set(keys);
                                                }
                                            } else {
                                                notification_message.set(resp.message);
                                                notification_type.set("error".to_string());
                                                show_notification.set(true);
                                            }
                                        }
                                        Err(e) => {
                                            notification_message.set(format!("Error creating API key: {}", e));
                                            notification_type.set("error".to_string());
                                            show_notification.set(true);
                                        }
                                    }
                                    creating_api_key.set(false);
                                });
                            },
                            if creating_api_key() { "Creating..." } else { "Create API Key" }
                        }
                    }

                    // Show newly created key
                    if let Some(ref plaintext) = newly_created_key() {
                        div { class: "mb-4 p-3 bg-yellow-50 border border-yellow-300 rounded-md",
                            div { class: "text-sm font-semibold text-yellow-800 mb-1",
                                "Save this key — it won't be shown again."
                            }
                            div { class: "flex gap-2 items-center",
                                code { class: "flex-1 text-xs font-mono bg-white px-2 py-1.5 border border-yellow-200 rounded break-all",
                                    "{plaintext}"
                                }
                                button {
                                    class: "text-xs px-2 py-1.5 bg-yellow-100 hover:bg-yellow-200 border border-yellow-300 rounded whitespace-nowrap",
                                    onclick: {
                                        let key_val = plaintext.clone();
                                        move |_| {
                                            let js = format!(
                                                "navigator.clipboard.writeText({:?}).catch(function(){{}})",
                                                key_val
                                            );
                                            let _ = document::eval(&js);
                                        }
                                    },
                                    "Copy"
                                }
                                button {
                                    class: "text-xs text-gray-400 hover:text-gray-600",
                                    onclick: move |_| newly_created_key.set(None),
                                    "Dismiss"
                                }
                            }
                        }
                    }

                    if api_keys_list().is_empty() {
                        div { class: "text-gray-500 text-sm text-center py-4", "No API keys yet." }
                    } else {
                        div { class: "flex flex-col gap-2 max-h-64 overflow-y-auto pr-1",
                            for key_info in api_keys_list() {
                                div {
                                    key: "{key_info.id}",
                                    class: "flex justify-between items-center px-3 py-2 border border-gray-200 rounded-md text-sm",
                                    div { class: "flex flex-col gap-0.5",
                                        div { class: "font-medium text-gray-900", "{key_info.name}" }
                                        div { class: "text-xs text-gray-500 font-mono", "{key_info.key_preview}" }
                                        div { class: "text-xs",
                                            if key_info.is_active {
                                                span { class: "text-green-600", "Active" }
                                            } else {
                                                span { class: "text-red-500", "Revoked" }
                                            }
                                        }
                                    }
                                    div { class: "flex gap-2",
                                        if key_info.is_active {
                                            button {
                                                class: "text-xs text-yellow-600 hover:text-yellow-800 px-2 py-1 border border-yellow-300 rounded",
                                                onclick: {
                                                    let kid = key_info.id.clone();
                                                    let gid = group_id();
                                                    move |_| {
                                                        let kid = kid.clone();
                                                        let gid = gid.clone();
                                                        spawn(async move {
                                                            if let Ok(true) = admin_revoke_api_key(kid).await {
                                                                if let Ok(keys) = admin_get_api_keys(gid).await {
                                                                    api_keys_list.set(keys);
                                                                }
                                                            }
                                                        });
                                                    }
                                                },
                                                "Revoke"
                                            }
                                        }
                                        button {
                                            class: "text-xs text-red-500 hover:text-red-700 px-2 py-1 border border-red-200 rounded",
                                            onclick: {
                                                let kid = key_info.id.clone();
                                                let gid = group_id();
                                                move |_| {
                                                    let kid = kid.clone();
                                                    let gid = gid.clone();
                                                    spawn(async move {
                                                        if let Ok(true) = admin_delete_api_key(kid).await {
                                                            if let Ok(keys) = admin_get_api_keys(gid).await {
                                                                api_keys_list.set(keys);
                                                            }
                                                        }
                                                    });
                                                }
                                            },
                                            "Delete"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AdminCreateGroup() -> Element {
    AdminGroup(AdminGroupProps { id: None })
}

#[component]
pub fn AdminEditGroup(id: String) -> Element {
    let id_signal = use_signal(|| id);
    AdminGroup(AdminGroupProps {
        id: Some(id_signal),
    })
}
