# Polkapipe

A low level client library for Substrate chains, doing less by design than [subxt](https://github.com/paritytech/substrate-subxt) and [sube](https://github.com/virto-network/sube) (this is an opinionated fork of sube) with a big focus on few dependencies and portability so it can run in constrainted environments like the browser.

It does not touch the metadata and leaves everything as bytes. You should use some other crate such as desub to use metadata to decode the data (or [Scales](https://github.com/virto-network/scales) or the `scale-value` crate or polkadyn). If you want to cache the data you get back before decoding you can.

Polkapipe supports multiple backends under different feature flags like `http`, `http-web` or `ws`/`wss`, `ws-web`, and `smoldot-std` (a [smoldot](https://github.com/paritytech/smoldot) based light-node).

## Goals:

  * few dependencies (and work in browesr)
  * endever to give you as good an error message as we can get our hands on. (work in progress)
  * provides an interface in scale (hiding away any json or hex that might unfortunately happen under the covers. Hopefully one day direct scale responses can be got over the websocket rather than it all being text, and on that day not much should change for the client.)


## Wasm:

To compile for wasm:

```
cargo check --target wasm32-unknown-unknown --features ws-web --no-default-features
```

To test for wasm:
```
wasm-pack test --headless --firefox --no-default-features --features ws-web
```

## Changelog
0.8:
 * Used less async traits. Now use `PolkaPipe::<Backend>` rather than `Backend`.
 * use nightly async trait for rpc rather than the crate.
 * re-add submit
 * added subscribe state (TODO: would be nice to unsubscribe when `drop`ped).
0.7:
 * Removed ws_stream_wasm and used the underlying
WebSocket directly. Less deps. 
 * Made logging an opt in feature.
 * Backend not Send + Sync