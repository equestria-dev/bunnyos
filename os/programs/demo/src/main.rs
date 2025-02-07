#![no_std]
#![no_main]

use rstd::prelude::*;

#[russet_entry]
fn main() {
    println!("Hello world!");
    panic!("Woopsie!");
}
