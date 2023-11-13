use std::env;
use std::fs::read;
use std::fs::write;
use std::path::Path;
use std::fmt::Write;
// generated by `sqlx migrate build-script`
fn main() {
    // trigger recompilation when a new migration is added
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=src/commands/data/messages.txt");
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("messages.rs");
    let raw_messages = read("./src/data/messages.txt").unwrap();
    let messages = String::from_utf8_lossy(&raw_messages);
    let lines: Vec<&str> = messages.lines().collect();
    let line_count = lines.len();
    let data: String = lines.into_iter().fold(String::new(), |mut s, i| {
        let _ = write!(s, r##"r#"{}"#,"##, i);
        s
    });
    let data = format!("const MESSAGES: [&str; {}] = [{}];", line_count, data.strip_suffix(',').unwrap());
    write(dest_path, data).unwrap();
}
