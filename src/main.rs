#![allow(incomplete_features)]
#![feature(const_generics)]
#![feature(array_value_iter)]
#![feature(maybe_uninit_ref)]

pub mod board;
pub mod list;
pub mod moves;
pub mod threat;
mod web;

pub use board::Board;

fn main() {
    crate::web::run_webserver(Board::new());
}
