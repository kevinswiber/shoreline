use std::path::Path;

pub(crate) fn naming_cutover_bytes(relative: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/naming-cutover")
            .join(relative),
    )
    .unwrap_or_else(|error| panic!("read naming-cutover fixture {relative}: {error}"))
}

pub(crate) fn naming_cutover_contract_bytes(relative: &str) -> Vec<u8> {
    let mut bytes = naming_cutover_bytes(relative);
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
    }
    bytes
}
