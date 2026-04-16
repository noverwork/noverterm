// @generated automatically by Diesel CLI.

diesel::table! {
    settings (id) {
        id -> Integer,
        key -> Text,
        value -> Text,
    }
}
