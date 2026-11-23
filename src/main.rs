fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use diesel::prelude::*;
    use rust_api::models::{NewUser, User};
    use rust_api::schema::users;

    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(std::env::VarError::NotPresent) => "app.sqlite".to_owned(),
        Err(error) => return Err(error.into()),
    };
    let mut conn = rust_api::establish_connection(&database_url)?;

    diesel::insert_into(users::table)
        .values(NewUser { name: "Alice" })
        .execute(&mut conn)?;

    let user: User = users::table.first(&mut conn)?;
    println!("Inserted and queried user: {:?}", user);
    Ok(())
}
