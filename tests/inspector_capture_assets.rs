use std::fs;
use std::path::Path;

use serde_json::Value;
use sha2::{Digest, Sha256};

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn png_dimensions(bytes: &[u8]) -> (u32, u32) {
    assert!(
        bytes.len() >= 24,
        "asset is too short to contain a PNG header"
    );
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "asset is not a PNG");
    (
        u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
        u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
    )
}

#[test]
fn marketing_review_capture_is_neutral_and_integrity_checked() {
    let manifest_path = repo_root().join("assets/marketing/review-interface-capture.json");
    let manifest: Value = serde_json::from_slice(
        &fs::read(&manifest_path)
            .unwrap_or_else(|error| panic!("read {}: {error}", manifest_path.display())),
    )
    .expect("capture manifest is valid JSON");

    assert_eq!(
        manifest["schema"],
        "com.withpointbreak.review-interface-capture/v1"
    );

    let revision = manifest["source"]["revision"]
        .as_str()
        .expect("source revision is a string");
    let digest = revision
        .strip_prefix("rev:sha256:")
        .expect("source revision uses the full rev:sha256 form");
    assert_eq!(digest.len(), 64, "source revision digest is complete");
    assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));

    assert_eq!(
        manifest["source"]["track"],
        "example:marketing-review-proof"
    );
    assert_eq!(manifest["source"]["selected_assessment"], "accepted");
    assert_eq!(manifest["source"]["publicly_inspectable"], false);
    assert_eq!(
        manifest["source"]["redactions"].as_array().map(Vec::len),
        Some(0)
    );

    let writers = manifest["source"]["writer_actors"]
        .as_array()
        .expect("writer_actors is an array");
    assert!(!writers.is_empty(), "at least one writer actor is recorded");
    for writer in writers {
        let writer = writer.as_str().expect("writer actor is a string");
        assert!(writer.starts_with("actor:"), "invalid actor id: {writer}");
        let normalized = writer.to_ascii_lowercase();
        assert!(
            !normalized.contains('@'),
            "writer exposes an email: {writer}"
        );
        assert!(
            !normalized.contains("kswiber") && !normalized.contains("kevin"),
            "writer exposes a personal principal: {writer}"
        );
    }

    assert_eq!(manifest["capture"]["viewport"]["width"], 900);
    assert_eq!(manifest["capture"]["viewport"]["height"], 506);
    assert_eq!(manifest["capture"]["device_scale_factor"], 2);

    for theme in ["dark", "light"] {
        let asset = &manifest["assets"][theme];
        let relative_path = asset["path"].as_str().expect("asset path is a string");
        let bytes = fs::read(repo_root().join(relative_path)).expect("read capture asset");
        assert_eq!(png_dimensions(&bytes), (1800, 1012));
        assert_eq!(asset["width"], 1800);
        assert_eq!(asset["height"], 1012);
        assert_eq!(
            asset["sha256"].as_str().expect("asset digest is a string"),
            sha256(&bytes)
        );
    }
}
