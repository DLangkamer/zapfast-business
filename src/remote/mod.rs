//! ZapFast Desktop Remote Client module.
//!
//! Enables connecting the native desktop app to a ZapFast Server (Docker / LAN),
//! with auto-discovery, authentication, and remote multi-account sync.

pub mod client;
pub mod discovery;

pub use client::{RemoteClient, RemoteState, RemoteUser};
pub use discovery::{discover_local_servers, DiscoveredServer};
