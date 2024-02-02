use actix_web::{error, web, App, HttpResponse, HttpServer, Responder};
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, Pool},
};

pub mod models;
pub mod schema;

use models::*;

type PgPool = r2d2::Pool<ConnectionManager<diesel::pg::PgConnection>>;

async fn set_user(pool: web::Data<PgPool>, json: web::Json<NewUser>) -> actix_web::Result<impl Responder> {
    use self::schema::users::dsl::*;
    let rows_affected = web::block(move || {
        let mut conn = pool.get().expect("couldn't get db connection from pool");
        diesel::insert_into(users)
            .values(&json.into_inner())
            .execute(&mut conn)
    })
    .await?
    .map_err(error::ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().json(rows_affected))
}

async fn get_users(pool: web::Data<PgPool>) -> actix_web::Result<impl Responder> {
    use self::schema::users::dsl::*;
    let user_records = web::block(move || {
        let mut conn = pool.get().expect("couldn't get db connection from pool");
        users
            .select(User::as_select())
            .load(&mut conn)
            .expect("Error loading users")
    })
    .await?;
    Ok(HttpResponse::Ok().json(user_records))
}

async fn index() -> HttpResponse {
    HttpResponse::Ok()
        .body("GET /users/all\nor\nPOST /users/add")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let pg_url = std::env::var("PG__URL").expect("PG__URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(pg_url);
    let pool = Pool::builder()
        .test_on_check_out(true)
        .build(manager)
        .expect("couldn't build pool");
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/", web::get().to(index))
            .route("/users/add", web::post().to(set_user))
            .route("/users/all", web::get().to(get_users))
    })
    .bind(("0.0.0.0", 8080))?
    .run();
    println!("Server running");
    server.await
}
