# Polkapipe

A low level client library for Substrate chains, doing less by design than [subxt](https://github.com/paritytech/substrate-subxt) and [sube](https://github.com/virto-network/sube) (this is an opinionated fork of sube) with a big focus on few dependencies and portability so it can run in constrainted environments like the browser.

It does not touch the metadata and leaves everything as bytes. You should use some other crate such as desub to use metadata to decode the data (or [Scales](https://github.com/virto-network/scales) or the scale-value crate). If you want to cache the data you get back before decoding you now can.

When submitting extrinsics Sube only does that, it's your responsability to sign the payload with a different tool first(e.g. [libwallet](https://github.com/valibre-org/libwallet)) before you feed the extrinsic data to the library.

Polkapipe supports multiple backends under different feature flags like `http`, `http-web` or `ws`/`wss`, with [future support](https://github.com/virto-network/sube/milestone/3) for a [`smoldot`](https://github.com/paritytech/smoldot) based light-node.  
