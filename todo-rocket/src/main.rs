#[macro_use]
extern crate rocket;
use rocket::http::Status;
use rocket::response::status;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};

use rocket::State;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![create, list, change, remove])
        .manage(Arc::new(Mutex::new(AppState {
            id: 0,
            items: HashMap::new(),
        })))
}

struct AppState {
    id: u8,
    items: HashMap<u8, Item>,
}

#[derive(Serialize, Clone)]
// #[serde(crate = "rocket::serde")]
struct Item {
    title: String,
    done: bool,
}

#[post("/", data = "<payload>")]
async fn create(
    payload: Json<CreatePayload>,
    state: &State<Arc<Mutex<AppState>>>,
) -> status::Created<Json<CreatedResponse>> {
    let mut lstate = state.lock().unwrap();
    let id = lstate.id;
    lstate.id += 1;
    lstate.items.insert(
        id,
        Item {
            title: payload.title.clone(),
            done: false,
        },
    );
    return status::Created::new("/").body(Json(CreatedResponse { id }));
}

#[derive(Deserialize)]
// #[serde(crate = "rocket::serde")]
struct CreatePayload {
    title: String,
}

#[derive(Serialize)]
// #[serde(crate = "rocket::serde")]
struct CreatedResponse {
    id: u8,
}

#[get("/")]
async fn list(state: &State<Arc<Mutex<AppState>>>) -> Json<Vec<ListItem>> {
    let lstate = state.lock().unwrap();
    return Json(Vec::from_iter(lstate.items.iter().map(|(id, item)| {
        ListItem {
            id: *id,
            title: item.clone().title,
            done: item.clone().done,
        }
    })));
}

#[derive(Serialize)]
// #[serde(crate = "rocket::serde")]
struct ListItem {
    id: u8,
    title: String,
    done: bool,
}

#[patch("/<id>", data = "<payload>")]
async fn change(
    id: u8,
    payload: Json<ChangePayload>,
    state: &State<Arc<Mutex<AppState>>>,
) -> Status {
    let mut lstate = state.lock().unwrap();
    match lstate.items.get_mut(&id) {
        Some(item) => {
            if let Some(new_title) = payload.title.clone() {
                item.title = new_title;
            }
            if let Some(new_done) = payload.done {
                item.done = new_done;
            }
            return Status::NoContent;
        }
        None => Status::NotFound,
    }
}

#[derive(Deserialize)]
// #[serde(crate = "rocket::serde")]
struct ChangePayload {
    title: Option<String>,
    done: Option<bool>,
}

#[delete("/<id>")]
async fn remove(id: u8, state: &State<Arc<Mutex<AppState>>>) -> Status {
    let mut lstate = state.lock().unwrap();
    match lstate.items.remove(&id) {
        Some(_) => Status::NoContent,
        None => Status::NotFound,
    }
}
