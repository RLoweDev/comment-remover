use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).parent().unwrap().parent().unwrap().parent().unwrap();
    
    // Copy syntax_rules.json to the target directory
    fs::copy("syntax_rules.json", dest_path.join("syntax_rules.json"))
        .expect("Failed to copy syntax_rules.json to target directory");
} 