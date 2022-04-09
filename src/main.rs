extern crate rts_rs;

use rts_rs::game;
use rts_rs::map::{MapConfig, MapType};

fn main() {
    let args = std::env::args();
    let args: Vec<String> = args.collect();
    let map_config = if args.get(1).map(String::as_str) == Some("loadtest") {
        MapConfig::Type(MapType::LoadTest)
    } else if args.get(1).map(String::as_str) == Some("empty") {
        MapConfig::Type(MapType::Empty)
    } else if args.get(1).map(String::as_str) == Some("small") {
        MapConfig::Type(MapType::Small)
    } else if let Some(filename) = args.get(1) {
        let map_file_path = format!("/maps/{}", filename);
        MapConfig::FromFile(Box::new(map_file_path))
    } else {
        MapConfig::Type(MapType::Medium)
    };

    game::run(map_config).expect("game crashed");
}
