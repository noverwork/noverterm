// @generated automatically by Diesel CLI.

diesel::table! {
    host_groups (id) {
        id -> Text,
        name -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    host_snippets (id) {
        id -> Text,
        host_id -> Text,
        title -> Text,
        body -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    settings (key) {
        key -> Text,
        value -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    ssh_hosts (id) {
        id -> Text,
        name -> Text,
        host -> Text,
        port -> Integer,
        username -> Text,
        ssh_key_id -> Nullable<Text>,
        password -> Nullable<Text>,
        group_id -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    trusted_hosts (host, port) {
        host -> Text,
        port -> Integer,
        algorithm -> Text,
        fingerprint -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    ssh_keys (id) {
        id -> Text,
        name -> Text,
        kind -> Text,
        fingerprint -> Nullable<Text>,
        private_key -> Text,
        passphrase -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(host_snippets -> ssh_hosts (host_id));
diesel::joinable!(ssh_hosts -> host_groups (group_id));
diesel::joinable!(ssh_hosts -> ssh_keys (ssh_key_id));

diesel::allow_tables_to_appear_in_same_query!(
    host_groups,
    host_snippets,
    settings,
    ssh_hosts,
    ssh_keys,
    trusted_hosts,
);
