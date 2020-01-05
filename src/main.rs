#![feature(proc_macro_hygiene)]
#![feature(vec_remove_item)]

#[macro_use] extern crate rocket;
// TODO: can't the json! macro be imported using the rust 2018 macro importing rules?
#[macro_use] extern crate rocket_contrib;

use rand::{
    distributions::{
        Distribution,
        Uniform,
    },
    seq::SliceRandom,
};
use rocket::{
    State,
};
use rocket_contrib::{
    json::{
        Json,
        JsonValue,
        serde::{Serialize, Deserialize},
    },
    serve::StaticFiles,
};
use std::sync::{
    mpsc,
    Mutex,
};

struct GameState {
    deck: Mutex<Vec<u8>>,
    cards: Mutex<Vec<u8>>,
    notify: Mutex<Vec<mpsc::Sender<()>>>,
}

impl GameState {
    fn deal_game(&self) {
        let mut deck = self.deck.lock().unwrap();
        let mut cards = self.cards.lock().unwrap();
        *deck = (1..64).collect();
        deck.shuffle(&mut rand::thread_rng());
        *cards = Vec::new();
    }

    fn draw_hand(&self) {
        let mut deck = self.deck.lock().unwrap();
        let mut cards = self.cards.lock().unwrap();

        while deck.len() > 0 && cards.len() < 7 {
            cards.push(deck.pop().expect("no cards left in deck when deck.len() > 0"))
        }
    }
}

#[get("/cards")]
fn cards(game_state: State<GameState>) -> Json<Vec<u8>> {
    let cards = game_state.cards.lock().unwrap();
    Json(cards.clone())
}

// TODO: the Option should make the cards array nil in the json, so we don't need the "update"
// parameter. Or maybe it should only return a bool and call the /cards endpoint for the new cards?
#[derive(Serialize, Deserialize)]
struct UpdateMessage {
    update: bool,
    cards: Option<Vec<u8>>,
}

#[get("/update")]
fn update(game_state: State<GameState>) -> Json<UpdateMessage> {
    let (tx, rx) = mpsc::channel();

    {
        let mut notifies = game_state.notify.lock().unwrap();
        notifies.push(tx);
    }

    let res = if let Err(e) = rx.recv() {
        println!("Error unlocking: {}", e);
        UpdateMessage { update: false, cards: None }
    } else {
        let cards = game_state.cards.lock().unwrap();
        UpdateMessage { update: true, cards: Some(cards.clone()) }
    };

    Json(res)
}

#[get("/new")]
fn new(game_state: State<GameState>) -> JsonValue {
    game_state.deal_game();

    game_state.draw_hand();

    json!({ "ok": true })
}

// TODO: use Optional to imply null instead of storing ok value
#[derive(Serialize)]
struct SubmitResponse {
    ok: bool, // true => success, false => error
    msg: String,
    success: Option<bool>, // None when ok is false
}

#[post("/submit", format = "application/json", data = "<submitted>")]
fn submit(submitted: Json<Vec<u8>>, game_state: State<GameState>) -> Json<SubmitResponse> {
    // TODO: check if cards are in hand
    let xor = submitted.iter().fold(0, |acc, card| acc ^ card);
    println!("xor of {:?} is {}", *submitted, xor);

    if xor == 0 {
        {
            let mut cards = game_state.cards.lock().unwrap();
            for card in submitted.iter() {
                cards.remove_item(card);
            }
        }

        game_state.draw_hand();

        {
            let mut notify = game_state.notify.lock().unwrap();
            for tx in notify.drain(..) {
                if let Err(e) = tx.send(()) {
                    return Json(SubmitResponse {
                        ok: false,
                        msg: format!("Notifies failed: {:?}", e),
                        success: None,
                    });
                }
            }
        }

        Json(SubmitResponse {
            ok: true,
            msg: "was a match!".to_string(),
            success: Some(true),
        })
    } else {
        Json(SubmitResponse {
            ok: true,
            msg: "not a match".to_string(),
            success: Some(false),
        })
    }
}

fn main() {
    let game_state = GameState {
        cards: Mutex::new(Vec::new()),
        deck: Mutex::new(Vec::new()),
        notify: Mutex::new(Vec::new()),
    };

    rocket::ignite()
        .mount("/", routes![cards, update, new, submit])
        .mount("/", StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/static")))
        .manage(game_state)
        .launch();
}
