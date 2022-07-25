# { pkgs ? import <nixpkgs> {} }:
let
  mozillaOverlay =
    import (builtins.fetchGit {
      url = "https://github.com/mozilla/nixpkgs-mozilla.git";
      rev = "e1f7540fc0a8b989fb8cf701dc4fd7fc76bcf168";
    });
  nixpkgs = import <nixpkgs> { overlays = [ mozillaOverlay ]; };
  rust-nightly = with nixpkgs; ((rustChannelOf { date = "2022-05-27"; channel = "nightly"; }).rust.override {
    extensions = [
	  "rust-src"
	];
    targets = [
      "wasm32-unknown-unknown"
    ];
  });
in
with nixpkgs; mkShell {
  nativeBuildInputs = [
    pkgconfig
    trunk # wasm only
    wasm-bindgen-cli # wasm only
    binaryen # wasm only
    clang lld # To use lld linker
  ];
  buildInputs = [
    libspnav udev alsaLib vulkan-loader
    xlibsWrapper xorg.libXcursor xorg.libXrandr xorg.libXi # To use x11 feature
    libxkbcommon wayland # To use wayland feature
    libspnav
    rust-nightly
  ];
  shellHook = ''export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath [
    udev alsaLib vulkan-loader
    libxkbcommon wayland # To use wayland feature
  ]}"'';
}
