extern crate rts_rs;

use rts_rs::game;
use rts_rs::maps::MapType;

fn main() {
    let args = std::env::args();
    let args: Vec<String> = args.collect();
    let map_type = if args.get(1).map(String::as_str) == Some("loadtest") {
        MapType::LoadTest
    } else {
        MapType::Small
    };

    println!("Running map: {:?}", map_type);
    game::run(map_type).expect("game crashed");
}
