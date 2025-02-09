#![no_std]
#![no_main]

use rstd::dbg;
use rstd::prelude::*;

#[russet_entry]
fn main() {
    eprintln!("Hello world!");
    let h = dbg!(1 + 1) + 1;
    panic!("Woopsie! {h}");
}
