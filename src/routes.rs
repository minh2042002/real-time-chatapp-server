use std::time::Instant;

use actix::*;
use actix_files::NamedFile;
use actix_web::{get, post, put, web, Error, HttpRequest, HttpResponse, Responder};
use actix_web_actors::ws;

use diesel::{
    prelude::*,
    r2d2::{self, ConnectionManager},
};
use serde_json::json;

use crate::models;
use crate::server;
use crate::session;
use crate::db;

type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

pub async fn index() -> impl Responder {
    NamedFile::open_async("../client/static/index.html")
        .await
        .unwrap()
}

pub async fn chat_server(
    req: HttpRequest,
    stream: web::Payload,
    pool: web::Data<DbPool>,
    srv: web::Data<Addr<server::ChatServer>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        session::WsChatSession {
            id: 0,
            hb: Instant::now(),
            room: "main".to_string(),
            name: None,
            addr: srv.get_ref().clone(),
            db_pool: pool,
        },
        &req,
        stream,
    )
}

#[get("/users")]
pub async fn get_users(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    let users = web::block(move || {
        let mut conn = pool.get()?;
        db::get_all_users(&mut conn)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    if !users.is_empty() {
        Ok(HttpResponse::Ok().json(users))
    } else {
        let res = HttpResponse::NotFound().body(
            json!({
                "error": 404,
                "message": "No user available at the moment.",
            })
            .to_string(),
        );
        Ok(res)
    }
}

#[post("/users/create")]
pub async fn create_user(
    pool: web::Data<DbPool>,
    form: web::Json<models::NewUser>,
) -> Result<HttpResponse, Error> {
    let user = web::block(move || {
        let mut conn = pool.get()?;
        db::insert_new_user(&mut conn, &form.username, &form.phone)
    })
    .await?
    .map_err(actix_web::error::ErrorUnprocessableEntity)?;

    Ok(HttpResponse::Ok().json(user))
}

#[get("/users/{user_id}")]
pub async fn get_user_by_id(
    pool: web::Data<DbPool>,
    id: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let user_id = id.to_owned();
    let user = web::block(move || {
        let mut conn = pool.get()?;
        db::find_user_by_uid(&mut conn, user_id)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    if let Some(user) = user {
        Ok(HttpResponse::Ok().json(user))
    } else {
        let res = HttpResponse::NotFound().body(
            json!({
                "error": 404,
                "message": format!("No user found with phone: {id}")
            })
            .to_string(),
        );
        Ok(res)
    }
}

#[get("/users/phone/{user_phone}")]
pub async fn get_user_by_phone(
    pool: web::Data<DbPool>,
    phone: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let user_phone = phone.to_string();
    let user = web::block(move || {
        let mut conn = pool.get()?;
        db::find_user_by_phone(&mut conn, user_phone)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    if let Some(user) = user {
        Ok(HttpResponse::Ok().json(user))
    } else {
        let res = HttpResponse::NotFound().body(
            json!({
                "error": 404,
                "message": format!("No user found with phone: {}", phone.to_string())
            })
            .to_string(),
        );
        Ok(res)
    }
}

#[get("/conversations/{room_id}")]
pub async fn get_conversation_by_id(
    pool: web::Data<DbPool>,
    rid: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let room_id = rid.to_string();
    let conversations = web::block(move || {
        let mut conn = pool.get()?;
        db::get_conversation_by_room_id(&mut conn, room_id)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    if let Some(data) = conversations {
        Ok(HttpResponse::Ok().json(data))
    } else {
        let res = HttpResponse::NotFound().body(
            json!({
                "error": 404,
                "message": format!("No conversation with room_id: {}", rid.to_string())
            })
            .to_string(),
        );
        Ok(res)
    }
}

#[get("/rooms")]
pub async fn get_rooms(
    pool: web::Data<DbPool>
) -> Result<HttpResponse, Error> {
    let rooms = web::block(move || {
        let mut conn = pool.get()?;
        db::get_all_rooms(&mut conn)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    if !rooms.is_empty() {
        Ok(HttpResponse::Ok().json(rooms))
    } else {
        let res = HttpResponse::NotFound().body(
            json!({
                "error": 404,
                "message": "No rooms available at the moment.",
            })
            .to_string(),
        );
        Ok(res)
    }
}

#[get("/rooms/user/{uid}")]
pub async fn get_room_by_uid(
    pool: web::Data<DbPool>,
    uid: web::Path<String>, 
) -> Result<HttpResponse, Error> {
    let user_id = uid.to_owned();

    let rooms = web::block(move || {
        let mut conn = pool.get()?;
        db::get_rooms_by_uid(&mut conn, user_id)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    if !rooms.is_empty() {
        Ok(HttpResponse::Ok().json(rooms))
    } else {
        let res = HttpResponse::NotFound().body(
            json!({
                "error": 404,
                "message": "No rooms available at the moment.",
            })
            .to_string(),
        );
        Ok(res)
    }
}

#[get("/rooms/prepare/{user_id}")]
pub async fn prepare_room(
    pool: web::Data<DbPool>,
    uid: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let user_id = uid.to_owned();

    if user_id.eq("undefined") {
        let res = HttpResponse::BadRequest().body(
            json!({
                "error": 400,
                "message": "No user_id available at the moment.",
            })
            .to_string(),
        );
        Ok(res)
    } else {
        let rooms = web::block(move || {
            let mut conn = pool.get()?;
            db::insert_list_room(&mut conn, user_id)
        })
        .await?
        .map_err(actix_web::error::ErrorInternalServerError)?;
    
        if !rooms.is_empty() {
            Ok(HttpResponse::Ok().json(rooms))
        } else {
            let res = HttpResponse::NotFound().body(
                json!({
                    "error": 404,
                    "message": "No room available at the moment.",
                })
                .to_string(),
            );
            Ok(res)
        }
    }
}

#[put("/rooms/update")]
pub async fn update_room(
    pool: web::Data<DbPool>,
    ur: web::Json<models::UpdateRoom>,
) -> Result<HttpResponse, Error> {
    let room = ur.to_owned();

    let new_room = web::block(move || {
        let mut conn = pool.get()?;
        db::update_last_message_in_room(&mut conn, room.id, room.last_message)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    if let Some(data) = new_room {
        Ok(HttpResponse::Ok().json(data))
    } else {
        let res = HttpResponse::NotFound().body(
            json!({
                "error": 404,
                "message": format!("No room with room_id: {}", ur.id)
            })
            .to_string(),
        );
        Ok(res)
    }
}