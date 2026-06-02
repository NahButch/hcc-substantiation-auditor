//! Agent binary entry point. The reasoning/verify→self-correct loop lands in
//! Phase 2+; for now this just confirms the engine dependency is wired up.

use engine::ModelVersion;

fn main() {
    println!("HCC Substantiation Auditor — agent v{}", env!("CARGO_PKG_VERSION"));
    println!("engine model: {}", engine::V28::NAME);
}
