//! Integration tests for the Spectrum XML import CLI.

use std::{fs, process::Command};

#[test]
fn imports_spectrum_xml_to_domers_toml_file() {
    let output = std::env::temp_dir().join(format!(
        "domers-{}-{}.toml",
        std::process::id(),
        "spectrum-import"
    ));
    let input = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/config/spectrum_default_config.xml"
    );

    let status = Command::new(env!("CARGO_BIN_EXE_domers"))
        .args([
            "import-spectrum-xml",
            input,
            output.to_str().expect("valid temp path"),
        ])
        .status()
        .expect("domers should run");

    assert!(status.success());

    let toml = fs::read_to_string(&output).expect("output toml should exist");
    assert!(toml.contains("[dome]"));
    assert!(toml.contains("[stage]"));
    assert!(toml.contains("[madmom]"));

    let _ = fs::remove_file(output);
}

#[test]
fn spectrum_xml_import_matches_checked_example_config() {
    let output = std::env::temp_dir().join(format!(
        "domers-{}-{}.toml",
        std::process::id(),
        "example-golden"
    ));
    let input = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/config/spectrum_default_config.xml"
    );
    let expected = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/domers.toml");

    let status = Command::new(env!("CARGO_BIN_EXE_domers"))
        .args([
            "import-spectrum-xml",
            input,
            output.to_str().expect("valid temp path"),
        ])
        .status()
        .expect("domers should run");

    assert!(status.success());

    let generated = fs::read_to_string(&output).expect("output toml should exist");
    let checked = fs::read_to_string(expected).expect("example config should exist");
    assert_eq!(generated, checked);

    let _ = fs::remove_file(output);
}
