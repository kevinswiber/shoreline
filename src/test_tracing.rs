use std::sync::Once;

use tracing_subscriber::EnvFilter;

static INIT: Once = Once::new();

#[ctor::ctor]
fn init() {
    init_test_tracing();
}

fn init_test_tracing() {
    INIT.call_once(|| {
        let filter = std::env::var("RUST_LOG")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "off".to_owned());
        let filter = EnvFilter::try_new(filter).unwrap_or_else(|_| EnvFilter::new("off"));
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(false)
            .with_test_writer()
            .finish();

        let _ = tracing::subscriber::set_global_default(subscriber);
    });
}
