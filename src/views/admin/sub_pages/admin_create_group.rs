#![allow(non_snake_case)]

use crate::backend::server_functions::{
    AddGroupMemberRequest, CreateGroupRequest, DeleteGroupRequest, GetGroupRequest,
    RemoveGroupMemberRequest, SearchUsersRequest, UpdateGroupRequest, admin_add_group_member,
    admin_create_group, admin_delete_group, admin_get_group, admin_remove_group_member,
    admin_search_users, admin_update_group, GroupMember, UserSearchResult,
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
        if group_resource().is_some_and(|r| r.is_err())
            || group_resource()
                .is_some_and(|r| r.as_ref().is_ok_and(|resp| !resp.success))
        {
            return rsx! {
                div { class: "text-red-500", "Error loading group." }
            };
        } else {
            return rsx! { div { "Loading..." } };
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
