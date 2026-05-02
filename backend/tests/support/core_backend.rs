// These source modules are path-included into integration test crates.
#[allow(dead_code)]
#[path = "../../src/domain/model.rs"]
pub(crate) mod model;
#[allow(dead_code)]
#[path = "../../src/domain/query.rs"]
pub(crate) mod query;
#[allow(dead_code)]
#[path = "../../src/storage/mod.rs"]
pub(crate) mod store;
