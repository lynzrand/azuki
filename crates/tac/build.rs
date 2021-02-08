use anyhow::Result;
use std::env;

fn main() -> Result<()> {
    c_bindgen();
    Ok(())
}

fn c_bindgen() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    cbindgen::Builder::new()
        .with_language(cbindgen::Language::C)
        .with_crate(crate_dir)
        .with_include_guard("LIB_AZUKI_TAC_H")
        .with_item_prefix("Azuki_")
        .with_std_types(true)
        .with_pragma_once(true)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("lib_azuki_tac.h");
}
