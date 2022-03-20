extern crate rts_rs;

use rts_rs::game;

fn main() {
    println!("Running...");
    game::run().expect("game crashed");
}
