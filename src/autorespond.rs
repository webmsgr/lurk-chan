use rand::prelude::*;
use rand::thread_rng;
include!(concat!(env!("OUT_DIR"), "/messages.rs"));
pub fn message() -> &'static str {
    *MESSAGES.choose(&mut thread_rng()).unwrap()
}
