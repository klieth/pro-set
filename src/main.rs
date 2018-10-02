extern crate chrono;
extern crate fern;
extern crate hyper;
extern crate iron;
extern crate log;
extern crate persistent;
extern crate rand;
extern crate router;
extern crate rustc_serialize;

use iron::{Iron,Request,Response,IronResult,IronError,status,Plugin,Chain};
use iron::mime::Mime;
use iron::typemap::Key;
use persistent::State;
use router::Router;
use rand::distributions::{Distribution,Uniform};
use rustc_serialize::json;
use rustc_serialize::json::Json;
use std::io::prelude::*;
use std::fs::File;
use std::sync::{Arc,Mutex};
use std::sync::mpsc::{channel,Sender};

#[derive(RustcEncodable)]
struct SubmitResponse {
    ok: bool, // true => success, false => error
    msg: String,
    success: Option<bool>, // None when ok is false
}
#[derive(RustcEncodable)]
struct UpdateMessage {
    update: bool,
    cards: Option<Vec<u8>>,
}

#[derive(Copy,Clone)]
struct CardData;
impl Key for CardData {
    type Value = Vec<u8>;
}
#[derive(Copy,Clone)]
struct DeckData;
impl Key for DeckData {
    type Value = Vec<u8>;
}
#[derive(Copy,Clone)]
struct NotifyLocks;
impl Key for NotifyLocks {
    type Value = Vec<Arc<Mutex<Sender<Vec<u8>>>>>;
}

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply()?;
    Ok(())
}

fn main() {
    setup_logger().unwrap();

    let card_data = State::<CardData>::both(Vec::new());
    let deck_data = State::<DeckData>::both( (1..64).collect() );
    let notify_locks = State::<NotifyLocks>::both( Vec::new() );

    let mut c_new_game = Chain::new(new_game);
    c_new_game.link(card_data.clone());
    c_new_game.link(deck_data.clone());

    let mut c_cards = Chain::new(cards);
    c_cards.link(card_data.clone());
    c_cards.link(deck_data.clone());

    let mut c_submit = Chain::new(submit);
    c_submit.link(card_data.clone());
    c_submit.link(deck_data.clone());
    c_submit.link(notify_locks.clone());

    let mut c_update = Chain::new(update);
    c_update.link(notify_locks.clone());

    let mut router = Router::new();
    router.get("/", index, "index");
    router.get("/new", c_new_game, "new_game");
    router.get("/cards", c_cards, "cards");
    router.post("/submit", c_submit, "submit");
    router.get("/update", c_update, "update");

    Iron::new(router).http("0.0.0.0:3000").unwrap();

    fn draw_card(deck: &mut Vec<u8>) -> u8 {
        let between = Uniform::new(0, deck.len());
        let mut rng = rand::thread_rng();
        deck.remove( between.sample(&mut rng) )
    };

    fn index(_: &mut Request) -> IronResult<Response> {
        let mut f = try!(File::open("index.html").map_err(|e| IronError::new(e, format!("ERROR couldn't find file"))));
        let mut s = String::new();
        try!(f.read_to_string(&mut s).map_err(|e| IronError::new(e, format!("ERROR couldn't read file"))));
        let content_type = "text/html".parse::<Mime>().unwrap();
        Ok( Response::with( (content_type, status::Ok, s) ) )
    }

    fn new_game(req: &mut Request) -> IronResult<Response> {
        let c_lock = try!(req.get::<State<CardData>>().map_err(|e| IronError::new(e, format!("ERROR couldn't get c_lock"))) );
        let mut card_data = c_lock.write().unwrap();
        let d_lock = try!(req.get::<State<DeckData>>().map_err(|e| IronError::new(e, format!("ERROR couldn't get d_lock"))) );
        let mut deck_data = d_lock.write().unwrap();
        // TODO send notify with empty card list
        *card_data = Vec::new();
        *deck_data = (1..64).collect();
        for _ in 0..7 {
            card_data.push(draw_card(&mut deck_data));
        }
        Ok( Response::with( (status::Ok, "{\"ok\":true}") ) )
    }

    fn cards(req: &mut Request) -> IronResult<Response> {
        let lock = try!(req.get::<State<CardData>>().map_err(|e| IronError::new(e, format!("ERROR couldn't get lock"))) );
        let card_data = lock.read().unwrap();
        let encoded = json::encode(&*card_data).unwrap();
        Ok(Response::with( (status::Ok, encoded) ))
    }

    fn submit(req: &mut Request) -> IronResult<Response> {
        // grab card data
        let c_lock = try!(req.get::<State<CardData>>().map_err(|e| IronError::new(e, format!("ERROR couldn't get c_lock"))) );
        let mut card_data = c_lock.write().unwrap();
        let d_lock = try!(req.get::<State<DeckData>>().map_err(|e| IronError::new(e, format!("ERROR couldn't get d_lock"))) );
        let mut deck_data = d_lock.write().unwrap();
        println!("getting notify lock");
        let n_lock = try!(req.get::<State<NotifyLocks>>().map_err(|e| IronError::new(e, format!("ERROR couldn't get n_lock"))) );
        let mut notify_locks = n_lock.write().unwrap();
        println!("got it");
        // grab body
        let mut body = String::new();
        req.body.read_to_string(&mut body).map_err(|e| IronError::new(e, iron::status::BadRequest))?;
        let cards = Json::from_str(&body).unwrap();
        let cards = cards.as_array().unwrap();
        let cards : Vec<u8> = cards.iter().map(|c| c.as_u64().unwrap() as u8).collect();
        // TODO check if cards are in current set
        let mut xor = 0u8;
        for card in cards.iter() {
            xor ^= *card;
        }
        println!("xor of {:?} is {}", cards, xor);
        if xor == 0 {
            card_data.retain(|c| !cards.contains(c));
            println!("card_data after remove: {:?}", *card_data);
            while deck_data.len() > 0 && card_data.len() < 7 {
                card_data.push(draw_card(&mut deck_data));
            }
            println!("card_data after draw: {:?}", *card_data);
            while notify_locks.len() > 0 {
                let tx = notify_locks.pop().unwrap();
                let tx = tx.lock().unwrap();
                match tx.send(cards.clone()) {
                    Ok(_) => println!("sent"),
                    Err(e) => println!("couldn't send: {}", e),
                }
            }
        }
        let sr = SubmitResponse { ok : true, msg : format!("was a match: {}", xor == 0), success : Some(xor == 0) };
        let sr = json::encode(&sr).unwrap();
        Ok( Response::with( (status::Ok, sr ) ) )
    }

    fn update(req: &mut Request) -> IronResult<Response> {
        let (tx, rx) = channel();
        {
            let n_lock = try!(req.get::<State<NotifyLocks>>().map_err(|e| IronError::new(e, format!("ERROR couldn't get n_lock"))) );
            let mut notify_locks = n_lock.write().unwrap();
            notify_locks.push(Arc::new(Mutex::new(tx)));
        }
        let res = match rx.recv() {
            Ok(cards) => UpdateMessage { update: true, cards: Some(cards) },
            Err(e) => {
                println!("Error unlocking: {}", e);
                UpdateMessage { update: false, cards: None }
            },
        };
        Ok(Response::with( (status::Ok, json::encode(&res).unwrap()) ) )
    }
}
