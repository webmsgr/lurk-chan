use rand::prelude::*;
use rand::thread_rng;
const MESSAGES: [&str; 6] = [
    "Hey, oomfie! How can I help you?",
    "Oomfie always remember to:   Always be nice to others",
    "Every second you are not running, I'm getting closer.",
    "Lurk-Chan says: Remember the golden rule!",
    "Lurkbot's my oomfie!",
    "Pnog!",
];

pub fn message() -> &'static str {
    *MESSAGES.choose(&mut thread_rng()).unwrap()
}
