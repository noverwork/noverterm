diesel::table! {
    users (id) {
        id -> Integer,
        name -> Text,
        email -> Text,
        created_at -> Nullable<Timestamp>,
    }
}
