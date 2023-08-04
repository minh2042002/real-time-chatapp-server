use chrono::{DateTime, Utc};
use diesel::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    time::SystemTime,
};
use uuid::Uuid;

use crate::models::{Conversation, NewConversation, Room, RoomResponse, User};

type DbError = Box<dyn std::error::Error + Send + Sync>;

pub fn find_user_by_uid(conn: &mut SqliteConnection, uid: String) -> Result<Option<User>, DbError> {
    use crate::schema::users::dsl::*;

    let user = users
        .filter(id.eq(uid.to_string()))
        .first::<User>(conn)
        .optional()?;

    Ok(user)
}

pub fn get_conversation_by_room_id(
    conn: &mut SqliteConnection,
    rid: String,
) -> Result<Option<Vec<Conversation>>, DbError> {
    use crate::schema::conversations::dsl::*;

    let convo = conversations
        .filter(room_id.eq(rid))
        .load(conn)
        .optional()?;

    Ok(convo)
}

pub fn get_rooms_by_uid(
    conn: &mut SqliteConnection,
    uid: String,
) -> Result<Vec<RoomResponse>, DbError> {
    use crate::schema::rooms;
    use crate::schema::users;

    let user_id = uid.to_string();

    println!("user id: {}", user_id);
    let rooms_data: Vec<Room> = rooms::table.get_results(conn)?; // lấy dữ liệu từ bảng rooms trong database
    let mut data: Vec<Room> = Vec::new(); // lưu các room của user đang xét
    let mut ids = HashSet::new(); // lưu các id của từng user
    let mut rooms_map = HashMap::new(); // lưu trữ dữ liệu của rooms gồm key là room_id và value là danh sách user_id

    for room in &rooms_data {
        if room.participant_ids.contains(user_id.as_str()) {
            data.push(room.to_owned());
        }
    }
    for room in &data {
        let user_ids = room
            .participant_ids
            .split(",")
            .into_iter()
            .collect::<Vec<_>>();
        for id in user_ids.to_vec() {
            ids.insert(id.to_string());
        }
        rooms_map.insert(room.id.to_string(), user_ids.to_vec());
    }

    let ids = ids.into_iter().collect::<Vec<_>>();
    let users_data: Vec<User> = users::table
        .filter(users::id.eq_any(ids))
        .get_results(conn)?;
    let users_map: HashMap<String, User> = HashMap::from_iter(
        users_data
            .into_iter()
            .map(|item| (item.id.to_string(), item)),
    );

    let response_rooms: Vec<RoomResponse> = data.to_owned()
        .into_iter()
        .map(|room| {
            let users = rooms_map
                .get(&room.id.to_string())
                .unwrap()
                .into_iter()
                .map(|id| users_map.get(id.to_owned()).unwrap().clone())
                .collect::<Vec<_>>();
            return RoomResponse { room, users };
        })
        .collect::<Vec<_>>();
    Ok(response_rooms)
}

pub fn update_last_message_in_room(
    conn: &mut SqliteConnection,
    rid: String,
    lm: String,
) -> Result<Option<Room>, DbError> {
    use crate::schema::rooms::dsl::*;

    let  room_id = rid.to_owned();

    diesel::update(rooms.filter(id.eq(room_id)))
                                .set(last_message.eq(lm))
                                .execute(conn)?;

    let room = rooms
        .filter(id.eq(rid))
        .first::<Room>(conn)
        .optional()?;
    
    Ok(room)
}

pub fn find_user_by_phone(
    conn: &mut SqliteConnection,
    user_phone: String,
) -> Result<Option<User>, DbError> {
    use crate::schema::users::dsl::*;

    let user = users
        .filter(phone.eq(user_phone))
        .first::<User>(conn)
        .optional()?;

    Ok(user)
}

pub fn get_all_rooms(conn: &mut SqliteConnection) -> Result<Vec<RoomResponse>, DbError> {
    use crate::schema::rooms;
    use crate::schema::users;

    let rooms_data: Vec<Room> = rooms::table.get_results(conn)?;
    let mut ids = HashSet::new();
    let mut rooms_map = HashMap::new();
    let data = rooms_data.to_vec();
    for room in &data {
        let user_ids = room
            .participant_ids
            .split(",")
            .into_iter()
            .collect::<Vec<_>>();
        for id in user_ids.to_vec() {
            ids.insert(id.to_string());
        }
        rooms_map.insert(room.id.to_string(), user_ids.to_vec());
    }

    let ids = ids.into_iter().collect::<Vec<_>>();
    let users_data: Vec<User> = users::table
        .filter(users::id.eq_any(ids))
        .get_results(conn)?;
    let users_map: HashMap<String, User> = HashMap::from_iter(
        users_data
            .into_iter()
            .map(|item| (item.id.to_string(), item)),
    );

    let response_rooms: Vec<RoomResponse> = rooms_data
        .into_iter()
        .map(|room| {
            let users = rooms_map
                .get(&room.id.to_string())
                .unwrap()
                .into_iter()
                .map(|id| users_map.get(id.to_owned()).unwrap().clone())
                .collect::<Vec<_>>();
            return RoomResponse { room, users };
        })
        .collect::<Vec<_>>();
    Ok(response_rooms)
}

fn iso_date() -> String {
    let now = SystemTime::now();
    let now: DateTime<Utc> = now.into();
    return now.to_rfc3339();
}

pub fn get_all_users(conn: &mut SqliteConnection) -> Result<Vec<User>, DbError> {
    use crate::schema::users::dsl::*;

    let users_data = users.load::<User>(conn)?;

    Ok(users_data)
}
pub fn insert_new_user(conn: &mut SqliteConnection, nm: &str, pn: &str) -> Result<User, DbError> {
    use crate::schema::users::dsl::*;

    let new_user = User {
        id: Uuid::new_v4().to_string(),
        username: nm.to_owned(),
        phone: pn.to_owned(),
        created_at: iso_date(),
    };

    diesel::insert_into(users).values(&new_user).execute(conn)?;

    Ok(new_user)
}

pub fn insert_new_conversation(
    conn: &mut SqliteConnection,
    new: NewConversation,
) -> Result<Conversation, DbError> {
    use crate::schema::conversations::dsl::*;

    let new_conversation = Conversation {
        id: Uuid::new_v4().to_string(),
        user_id: new.user_id,
        room_id: new.room_id,
        content: new.message,
        created_at: iso_date(),
    };

    diesel::insert_into(conversations)
        .values(&new_conversation)
        .execute(conn)?;

    Ok(new_conversation)
}

pub fn insert_list_room(
    conn: &mut SqliteConnection,
    uid: String,
) -> Result<Vec<RoomResponse>, DbError> {
    use crate::schema::rooms::dsl::*;

    let user_id = uid.to_string();
    let users = get_all_users(conn);

    for user in users.unwrap().iter() {
        if !user.id.eq(&user_id) {
            let mut parti = user_id.to_string();
            parti.push(',');
            parti.push_str(&user.id.to_string());

            let new_room = Room {
                id: Uuid::new_v4().to_string(),
                name: user.username.to_string(),
                last_message: "Let's start the conversation".to_string(),
                participant_ids: parti,
                created_at: iso_date(),
            };

            diesel::insert_into(rooms).values(&new_room).execute(conn)?;
        }
    }

    

    let list = get_rooms_by_uid(conn, uid);

    list
}
