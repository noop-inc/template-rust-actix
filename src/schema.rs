// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Int8,
        #[max_length = 200]
        email -> Varchar,
        #[max_length = 200]
        first_name -> Varchar,
        #[max_length = 200]
        last_name -> Varchar,
        #[max_length = 50]
        username -> Varchar,
    }
}
