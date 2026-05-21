// @generated automatically by Diesel CLI.

diesel::table! {
    host_groups (id) {
        id -> Text,
        owner_id -> Text,
        name -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    host_snippets (id) {
        id -> Text,
        host_id -> Text,
        owner_id -> Text,
        title -> Text,
        body -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    ssh_hosts (id) {
        id -> Text,
        name -> Text,
        host -> Text,
        port -> Int4,
        username -> Text,
        ssh_key_id -> Nullable<Text>,
        encrypted_password -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        owner_id -> Text,
        group_id -> Nullable<Text>,
    }
}

diesel::table! {
    ssh_keys (id) {
        id -> Text,
        name -> Text,
        kind -> Text,
        fingerprint -> Nullable<Text>,
        encrypted_private_key -> Text,
        encrypted_passphrase -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        owner_id -> Text,
    }
}

diesel::table! {
    user_settings (id) {
        id -> Text,
        owner_id -> Text,
        key -> Text,
        value -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    users (id) {
        id -> Text,
        email -> Text,
        password_hash -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(host_groups -> users (owner_id));
diesel::joinable!(host_snippets -> users (owner_id));
diesel::joinable!(ssh_hosts -> users (owner_id));
diesel::joinable!(ssh_keys -> users (owner_id));
diesel::joinable!(user_settings -> users (owner_id));

diesel::allow_tables_to_appear_in_same_query!(
    host_groups,
    host_snippets,
    ssh_hosts,
    ssh_keys,
    user_settings,
    users,
);
