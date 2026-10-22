use sha2::{Digest, Sha256};
use std::{fs, path::Path};

pub const SOURCE_INPUTS: [&str; 10] = [
    "Cargo.toml",
    "build.rs",
    "src/lib.rs",
    "src/value.rs",
    "src/predicate.rs",
    "src/outcome.rs",
    "src/assessment.rs",
    "src/failure.rs",
    "src/task.rs",
    "src/support.rs",
];

pub fn source_hash(root: &Path) -> std::io::Result<String> {
    let mut digest = Sha256::new();
    for name in SOURCE_INPUTS {
        let contents = fs::read(root.join(name))?;
        digest.update((name.len() as u64).to_le_bytes());
        digest.update(name.as_bytes());
        digest.update((contents.len() as u64).to_le_bytes());
        digest.update(contents);
    }
    Ok(format!("{:x}", digest.finalize()))
}

#[cfg(not(test))]
fn main() {
    let root = std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo manifest directory");
    for name in SOURCE_INPUTS {
        println!("cargo:rerun-if-changed={name}");
    }
    let hash = source_hash(Path::new(&root)).expect("read semantic core source inputs");
    println!("cargo:rustc-env=SEMANTIC_CORE_SOURCE_HASH={hash}");
}
