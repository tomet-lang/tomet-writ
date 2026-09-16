{
  inputs,
  pkgs,
  stdenv,
  mkShell,

  twrit,
  tomet,
  tmtbook,
  ...
}:
let
  fenix = inputs.fenix.packages.${stdenv.hostPlatform.system};
  rust-toolchain = fenix.combine [
    (fenix.stable.withComponents [
      "cargo"
      "clippy"
      "rustc"
      "rust-src"
      # "rust-analyzer"
    ])
    fenix.targets.wasm32-unknown-unknown.stable.rust-std
    fenix.targets.wasm32-wasip2.stable.rust-std
  ];
in
mkShell rec {
  buildInputs = with pkgs; [
    twrit
    tomet
    tmtbook

    #[ CMake ]
    cmake
    ninja

    #[ Rust ]
    rust-toolchain
    cargo-edit
    cargo-outdated
    cargo-nextest
    wasm-bindgen-cli

    #[ Misc ]
    pkg-config
  ];

  PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
  ];

  shellHook = ''
    echo "🦀 Rust"
  '';
}
