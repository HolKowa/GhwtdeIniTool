use std::{env, fs, path::PathBuf};

const DEVELOPMENT_LICENSES: &str = "Third-Party Licenses\n====================\n\nThird-party license notices are generated and embedded in release builds.\n";

fn main() {
    println!("cargo:rerun-if-env-changed=GHWTDE_INCLUDE_THIRD_PARTY_LICENSES");
    println!("cargo:rerun-if-changed=resources/THIRD_PARTY_LICENSES.txt");

    let license_output = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by Cargo"))
        .join("third_party_licenses.txt");
    let licenses = if env::var("GHWTDE_INCLUDE_THIRD_PARTY_LICENSES").as_deref() == Ok("1") {
        fs::read_to_string("resources/THIRD_PARTY_LICENSES.txt")
            .expect("release builds require generated third-party licenses; run `pnpm tauri build`")
    } else {
        DEVELOPMENT_LICENSES.to_owned()
    };

    fs::write(license_output, licenses).expect("write embedded third-party licenses");
    tauri_build::build()
}
