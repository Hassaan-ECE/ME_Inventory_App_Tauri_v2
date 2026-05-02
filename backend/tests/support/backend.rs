// These source modules are path-included into multiple integration test crates.
#[allow(dead_code)]
#[path = "../../src/domain/model.rs"]
pub(crate) mod model;
#[allow(dead_code)]
#[path = "../../src/storage/mod.rs"]
pub(crate) mod store;
// The path-included sync module exposes a shared test_support surface used piecemeal by tests.
#[allow(dead_code, unused_imports)]
#[path = "../../src/sync/mod.rs"]
pub(crate) mod sync;
