mod board;
mod card;
mod column;
mod containers;

use axum::{
    Json,
    // extract::{Extension, Path},
    routing::{get, patch, post},
    Router,
    Extension, extract::Path, http::StatusCode,
};
use column::Column;
use tokio::signal;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use crate::board::*;
use crate::card::Card;
use crate::containers::MoveSpec;

struct AppState {
    boards: Vec<Board>,
}

#[tokio::main]
async fn main() {
    let app_state = Arc::new(Mutex::new(AppState { boards: Vec::new() }));

    // build our application with a route
    let app = Router::new()
        .route("/board/", post(create_board).get(list_boards))
        .route("/board/:board_id", get(fetch_board).patch(change_board).delete(remove_board))
        .route("/board/:board_id/column", post(create_column))
        .route("/board/:board_id/column/:column_id", patch(change_column).delete(remove_column))
        .route("/board/:board_id/column/:column_id/card", post(create_card))
        .route("/board/:board_id/column/:column_id/card/:card_id", patch(change_card).delete(remove_card))
        .layer(Extension(Arc::clone(&app_state)));

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => println!("Shutting down..."),
        _ = terminate => println!("Shutting down..."),
    }
}

async fn create_board(
    Extension(state): Extension<Arc<Mutex<AppState>>>,
    Json(payload): Json<CreatePayload>
) -> StatusCode {
    let mut lstate = state.lock().unwrap();
    lstate.boards.push(Board {title: payload.title, items: Vec::new()});
    return StatusCode::CREATED;
}

async fn create_column(
    Extension(state): Extension<Arc<Mutex<AppState>>>,
    Path(board_id): Path<usize>,
    Json(payload): Json<CreatePayload>
) -> StatusCode {
    let mut lstate = state.lock().unwrap();
    match lstate.boards.get_mut(board_id) {
        Some(board) => {
            board.add_item(Column {title: payload.title, items: Vec::new()});
            return StatusCode::CREATED;
        },
        None => StatusCode::NOT_FOUND
    }
}

async fn create_card(
    Extension(state): Extension<Arc<Mutex<AppState>>>,
    Path(board_id): Path<usize>,
    Path(column_id): Path<usize>,
    Json(payload): Json<CreatePayload>
) -> StatusCode {
    let mut lstate = state.lock().unwrap();
    match lstate.boards.get_mut(board_id) {
        Some(board) => {
            let done = board.update_item(column_id, |column| {
                let mut new_column = column.clone();
                new_column.add_item(Card {title: payload.title.clone()});
                return new_column;
            });

            return
                if done {
                    StatusCode::CREATED
                } else {
                    StatusCode::NOT_FOUND
                }
        },
        None => StatusCode::NOT_FOUND
    }
}

#[derive(Deserialize)]
struct CreatePayload {
    title: String,
}

async fn list_boards(
    Extension(state): Extension<Arc<Mutex<AppState>>>
) -> Json<Vec<ListBoardsItem>> {
    let lstate = state.lock().unwrap();
    return Json(Vec::from_iter(lstate
        .boards
        .iter()
        .map(|board| ListBoardsItem {title: board.clone().title})));
}

#[derive(Serialize)]
struct ListBoardsItem {
    title: String,
}

async fn fetch_board(
    Extension(state): Extension<Arc<Mutex<AppState>>>,
    Path(board_id): Path<usize>,
) -> Result<Json<FetchedBoard>, StatusCode> {
    let lstate = state.lock().unwrap();
    match lstate.boards.get(board_id) {
        Some(board) =>
            Ok(
                Json(
                    FetchedBoard {
                        title: board.title.clone(),
                        columns:
                            Vec::from_iter(board.items.iter().map(|column|
                              FetchedColumn {
                                title: column.title.clone(),
                                cards:
                                    Vec::from_iter(column.items.iter().map(|card|
                                      FetchedCard {
                                        title: card.title.clone(),
                                    }
                                )),
                            }
                        )),
                    }
                )
            )
        ,
        None => Err(StatusCode::NOT_FOUND)
    }
}

#[derive(Serialize)]
struct FetchedBoard {
    title: String,
    columns: Vec<FetchedColumn>,
}

#[derive(Serialize)]
struct FetchedColumn {
    title: String,
    cards: Vec<FetchedCard>,
}

#[derive(Serialize)]
struct FetchedCard {
    title: String,
}

async fn change_board(
    Extension(state): Extension<Arc<Mutex<AppState>>>,
    Path(board_id): Path<usize>,
    Json(payload): Json<ChangeBoardPayload>,
) -> Result<(), StatusCode> {
    let mut lstate = state.lock().unwrap();
    match lstate.boards.get_mut(board_id) {
        Some(board) => {
            if let Some(new_title) = payload.title {
                board.rename(new_title);
            }
            return Ok(());
        },
        None => Err(StatusCode::NOT_FOUND)
    }
}

#[derive(Deserialize)]
struct ChangeBoardPayload {
    title: Option<String>,
}

async fn change_column(
    Extension(state): Extension<Arc<Mutex<AppState>>>,
    Path(board_id): Path<usize>,
    Path(column_id): Path<usize>,
    Json(payload): Json<ChangeContainedPayload>,
) -> StatusCode {
    let mut lstate = state.lock().unwrap();
    match lstate.boards.get_mut(board_id) {
        Some(board) => {
            let done_inside =
                board.update_item(column_id, |column| {
                    let mut new_column = column.clone();
                    if let Some(new_title) = payload.title.clone() {
                        new_column.rename(new_title);
                    }
                    return new_column;
                });

            let done_outside =
                if let Some(move_to) = payload.move_to {
                    board.move_item(
                        column_id,
                        match move_to {
                            ChangeContainedMoveSpec::Beginning => MoveSpec::Beginning,
                            ChangeContainedMoveSpec::End => MoveSpec::End,
                            ChangeContainedMoveSpec::After {position} => MoveSpec::After(position),
                        }
                    )
                } else {
                    true
                };

            return if done_inside && done_outside {
                    StatusCode::NO_CONTENT
                } else {
                    StatusCode::NOT_FOUND
                }
        },
        None => StatusCode::NOT_FOUND
    }
}

async fn change_card(
    Extension(state): Extension<Arc<Mutex<AppState>>>,
    Path(board_id): Path<usize>,
    Path(column_id): Path<usize>,
    Path(card_id): Path<usize>,
    Json(payload): Json<ChangeContainedPayload>,
) -> StatusCode {
    let mut lstate = state.lock().unwrap();
    match lstate.boards.get_mut(board_id) {
        Some(board) => {
            let done =
                board.update_item(column_id, |column| {
                    let mut new_column = column.clone();
                    if let Some(new_title) = payload.title.clone() {
                        new_column.update_item(card_id, |card| card.rename(new_title.clone()));
                    }

                    if let Some(move_to) = &payload.move_to {
                        new_column.move_item(
                            card_id,
                            match move_to {
                                ChangeContainedMoveSpec::Beginning => MoveSpec::Beginning,
                                ChangeContainedMoveSpec::End => MoveSpec::End,
                                ChangeContainedMoveSpec::After {position} => MoveSpec::After(*position),
                            }
                        );
                    };
                    return new_column;
                });

            return if done {
                    StatusCode::NO_CONTENT
                } else {
                    StatusCode::NOT_FOUND
                }
        },
        None => StatusCode::NOT_FOUND
    }
}

#[derive(Deserialize)]
struct ChangeContainedPayload {
    title: Option<String>,
    move_to: Option<ChangeContainedMoveSpec>,
}

#[derive(Deserialize)]
pub enum ChangeContainedMoveSpec {
    Beginning,
    End,
    After { position: usize },
}

async fn remove_board(
    Extension(state): Extension<Arc<Mutex<AppState>>>,
    Path(board_id): Path<usize>,
) -> Result<(), StatusCode> {
    let mut lstate = state.lock().unwrap();
    if lstate.boards.len() < board_id {
        return Err(StatusCode::NOT_FOUND);
    }
    lstate.boards.remove(board_id);
    return Ok(());
}
async fn remove_column(
    Extension(state): Extension<Arc<Mutex<AppState>>>,
    Path(board_id): Path<usize>,
    Path(column_id): Path<usize>,
) -> StatusCode {
    let mut lstate = state.lock().unwrap();
    match lstate.boards.get_mut(board_id) {
        Some(board) =>
            if board.remove_item(column_id) {
                    StatusCode::NO_CONTENT
                } else {
                    StatusCode::NOT_FOUND
                },
        None => StatusCode::NOT_FOUND
    }
}


async fn remove_card(
    Extension(state): Extension<Arc<Mutex<AppState>>>,
    Path(board_id): Path<usize>,
    Path(column_id): Path<usize>,
    Path(card_id): Path<usize>,
) -> StatusCode {
    let mut lstate = state.lock().unwrap();
    match lstate.boards.get_mut(board_id) {
        Some(board) => {
            let done = board.update_item(column_id, |column| {
                let mut new_column = column.clone();
                new_column.remove_item(card_id);
                return new_column;
            });
            return if done {
                    StatusCode::NO_CONTENT
                } else {
                    StatusCode::NOT_FOUND
                };
        },
        None => StatusCode::NOT_FOUND
    }
}
