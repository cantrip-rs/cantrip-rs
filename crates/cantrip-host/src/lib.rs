//! The native engine as a sidecar for `Cantrip.Server` (docs/engine-design.md, "Interfaces"): any number of
//! games behind the JSON-lines protocol of docs/host-protocol.md, which `NativeRulesHost` (C#) drives.
//!
//! - `protocol` mirrors `Cantrip.Protocol` field for field (engine-independent).
//! - `handles` are the per-seat handle spaces: an engine object id never reaches a client.
//! - `view` is the per-seat projection of the position, `prompt` and `answer` translate the engine's requests and
//!   answers, `events` turns the engine's log into the entries the client shows.
//! - `session` is one hosted game; `serve` the requests about any number of them, which `bin/host.rs` reads from stdin
//!   and `cantrip-wasm` from the browser.

pub mod answer;
pub mod events;
pub mod handles;
pub mod prompt;
pub mod protocol;
pub mod serve;
pub mod session;
pub mod text;
pub mod view;
