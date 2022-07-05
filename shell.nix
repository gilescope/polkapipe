let
  mozillaOverlay =
    import (builtins.fetchGit {
      # TODO: revert to upstream after https://github.com/mozilla/nixpkgs-mozilla/pull/250
      url = "https://github.com/andresilva/nixpkgs-mozilla.git";
      rev = "7626aca57c20f3f6ee28cce8657147d9b358ea18";
    });
  nixpkgs = import <nixpkgs> { overlays = [ mozillaOverlay ]; };
  # was 2021-09-27
  rust-nightly = with nixpkgs; ((rustChannelOf { date = "2022-05-18"; channel = "nightly"; }).rust.override {
    extensions = [ "rust-src" ];
    targets = [ "wasm32-unknown-unknown" ];
  });
in
with nixpkgs; pkgs.mkShell {
  buildInputs = [
 curl.dev
 clang
    pkg-config
	openssl.dev
    rust-nightly
  ] ++ lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.Security
  ];

  RUST_SRC_PATH="${rust-nightly}/lib/rustlib/src/rust/src";
  LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
  PROTOC = "${protobuf}/bin/protoc";
  ROCKSDB_LIB_DIR = "${rocksdb}/lib";
}
