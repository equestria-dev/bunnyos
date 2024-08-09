#![no_std]
#![no_main]

use bstd::prelude::*;

#[bunny_entry]
fn main() {
    println!("Hello world!");
}
