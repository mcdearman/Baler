//! Finds the `textwrap` and `smawk` sources that Cargo fetched, for `main.rs`
//! to fingerprint: `src/` ports them by hand.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let mut sources = String::new();
    for (name, files) in [
        (
            "textwrap",
            &[
                "src/core.rs",
                "src/columns.rs",
                "src/fill.rs",
                "src/indentation.rs",
                "src/line_ending.rs",
                "src/options.rs",
                "src/refill.rs",
                "src/word_separators.rs",
                "src/word_splitters.rs",
                "src/wrap.rs",
                "src/wrap_algorithms.rs",
                "src/wrap_algorithms/optimal_fit.rs",
            ][..],
        ),
        ("smawk", &["src/lib.rs"][..]),
    ] {
        let dir = upstream_dir(name);
        if name == "textwrap" {
            println!("cargo:rustc-env=UPSTREAM_DIR={}", dir.display());
        }
        for file in files {
            let path = dir.join(file);
            sources.push_str(
                &std::fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display())),
            );
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
    std::fs::write(out.join("sources.rs.txt"), sources).unwrap();
    println!("cargo:rerun-if-changed=Cargo.toml");
}

/// Where Cargo put the package `name` this build depends on.
fn upstream_dir(name: &str) -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let manifest = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("Cargo.toml");
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(&manifest)
        .output()
        .expect("could not run `cargo metadata`");
    assert!(out.status.success(), "`cargo metadata` failed");
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let pkg = meta["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == name)
        .unwrap_or_else(|| panic!("{name} is not among the dependencies"));
    Path::new(pkg["manifest_path"].as_str().unwrap())
        .parent()
        .unwrap()
        .to_path_buf()
}
