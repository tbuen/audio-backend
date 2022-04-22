use std::process::Command;

fn main() {
    let output = Command::new("git")
        .arg("describe")
        .arg("--always")
        .arg("--tags")
        .arg("--dirty")
        .output()
        .unwrap();
    let version = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=VERSION={}", version);
}
