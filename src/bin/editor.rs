extern crate rts_rs;

use rts_rs::map_editor;

fn main() {
    map_editor::run("resources/maps/small.txt".to_owned()).expect("Map editor crashed");
}
