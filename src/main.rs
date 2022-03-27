extern crate rts_rs;

use rts_rs::data::MapType;
use rts_rs::game;

fn main() {
    let args = std::env::args();
    let args: Vec<String> = args.collect();
    let map_type = if args.get(1).map(String::as_str) == Some("loadtest") {
        MapType::LoadTest
    } else if args.get(1).map(String::as_str) == Some("empty") {
        MapType::Empty
    } else if args.get(1).map(String::as_str) == Some("small") {
        MapType::Small
    } else {
        MapType::Medium
    };

    println!("Running map: {:?}", map_type);
    game::run(map_type).expect("game crashed");
}
