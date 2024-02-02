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

mod errors {
    use actix_web::{HttpResponse, ResponseError};
    use deadpool_postgres::PoolError;
    use derive_more::{Display, From};
    use tokio_pg_mapper::Error as PGMError;
    use tokio_postgres::error::Error as PGError;

    #[derive(Display, From, Debug)]
    pub enum MyError {
        NotFound,
        PGError(PGError),
        PGMError(PGMError),
        PoolError(PoolError),
    }
    impl std::error::Error for MyError {}

    impl ResponseError for MyError {
        fn error_response(&self) -> HttpResponse {
            match *self {
                MyError::NotFound => HttpResponse::NotFound().finish(),
                MyError::PoolError(ref err) => {
                    HttpResponse::InternalServerError().body(err.to_string())
                }
                _ => HttpResponse::InternalServerError().finish(),
            }
        }
    }
}

mod db {
    use deadpool_postgres::Client;
    use tokio_pg_mapper::FromTokioPostgresRow;

    use crate::{errors::MyError, models::User};

    pub async fn get_users(client: &Client) -> Result<Vec<User>, MyError> {
        let stmt = include_str!("../sql/get_users.sql");
        let stmt = stmt.replace("$table_fields", &User::sql_table_fields());
        let stmt = client.prepare(&stmt).await.unwrap();

        let results = client
            .query(&stmt, &[])
            .await?
            .iter()
            .map(|row| User::from_row_ref(row).unwrap())
            .collect::<Vec<User>>();

        Ok(results)
    }

    pub async fn add_user(client: &Client, user_info: User) -> Result<User, MyError> {
        let _stmt = include_str!("../sql/add_user.sql");
        let _stmt = _stmt.replace("$table_fields", &User::sql_table_fields());
        let stmt = client.prepare(&_stmt).await.unwrap();

        client
            .query(
                &stmt,
                &[
                    &user_info.email,
                    &user_info.first_name,
                    &user_info.last_name,
                    &user_info.username,
                ],
            )
            .await?
            .iter()
            .map(|row| User::from_row_ref(row).unwrap())
            .collect::<Vec<User>>()
            .pop()
            .ok_or(MyError::NotFound) // more applicable for SELECTs
    }
}

mod handlers {
    use actix_web::{web, Error, HttpResponse};
    use deadpool_postgres::{Client, Pool};

    use crate::{db, errors::MyError, models::User};

    pub async fn get_users(db_pool: web::Data<Pool>) -> Result<HttpResponse, Error> {
        let client: Client = db_pool.get().await.map_err(MyError::PoolError)?;

        let users = db::get_users(&client).await?;

        Ok(HttpResponse::Ok().json(users))
    }

    pub async fn add_user(
        user: web::Json<User>,
        db_pool: web::Data<Pool>,
    ) -> Result<HttpResponse, Error> {
        let user_info: User = user.into_inner();

        let client: Client = db_pool.get().await.map_err(MyError::PoolError)?;

        let new_user = db::add_user(&client, user_info).await?;

        Ok(HttpResponse::Ok().json(new_user))
    }
}

use ::config::Config;
use actix_web::{web, App, HttpServer};
use dotenvy::dotenv;
use handlers::{add_user, get_users};
use tokio_postgres::NoTls;
use crate::config::ExampleConfig;

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
