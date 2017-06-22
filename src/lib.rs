// Copyright (c) 2017 Bailey Cosier <bailey@cosier.ca>

//! A simple JSON file database written in Rust.

extern crate uuid;
extern crate fs2;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

mod config;
pub mod db;


