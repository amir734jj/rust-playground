diesel::table! {
    users (id) {
        id -> Integer,
        age -> Integer,
        firstname -> Text,
        lastname -> Text,
        username -> Text,
        password_hash -> Nullable<Text>,
    }
}

diesel::table! {
    projects (id) {
        id -> Integer,
        user_id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    tokens (id) {
        id -> Integer,
        user_id -> Integer,
        token_hash -> Text,
        expires_at -> BigInt,
    }
}

diesel::joinable!(projects -> users (user_id));
diesel::joinable!(tokens -> users (user_id));
diesel::allow_tables_to_appear_in_same_query!(users, projects, tokens);
