# CanopyBus

CanopyBus is a Rust decoder for greenhouse and vertical-farm gateway traffic. It parses field bundles emitted by edge controllers, rebuilds fragmented streams, normalizes device topology, decodes packed sensor pages, evaluates small rule programs, and replays audit journals into a queryable archive model.

The crate is intentionally organized like an operational parser: parser stages live in separate modules, fuzz targets exercise distinct public entry points, and the seed corpus contains realistic gateway messages. The build uses only first-party Rust plus a local fuzz shim, so ClusterFuzzLite can build it without network access.
