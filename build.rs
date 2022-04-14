use std::process::Command;

fn main() {
    // Set git HEAD hash as environment variable
    let commit = match Command::new("git").args(["rev-parse", "HEAD"]).output()
    {
        Ok(output) => String::from_utf8(output.stdout).unwrap_or_default(),
        Err(_) => String::new()
    };
    println!("cargo:rustc-env=GIT_HASH={}",
        commit.chars().take(7).collect::<String>());
}