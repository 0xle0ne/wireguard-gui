{
  lib,
  stdenv,
  rustPlatform,
  fetchNpmDeps,
  cargo-tauri,
  glib-networking,
  libsoup_3,
  nodejs_24,
  npmHooks,
  libiconv,
  libayatana-appindicator,
  openssl,
  pkg-config,
  webkitgtk_4_1,
  wrapGAppsHook3,
  makeBinaryWrapper,
}:

let
  src = ../.;
  toml = (lib.importTOML ../src-tauri/Cargo.toml).package;
in
rustPlatform.buildRustPackage rec {
  pname = toml.name;
  inherit src;
  # Tauri config owns the app version; Cargo keeps its 0.0.0 placeholder.
  version = (builtins.fromJSON (builtins.readFile ../src-tauri/tauri.conf.json)).version;

  npmDeps = fetchNpmDeps {
    inherit pname version src;
    hash = "sha256-vYEInkuDy0kq7FWNzjdD6bXqZ04/ljmCzIJyrRnzq38=";
  };

  cargoRoot = "src-tauri";

  cargoLock.lockFile = "${src}/src-tauri/Cargo.lock";

  buildAndTestSubdir = cargoRoot;

  nativeBuildInputs =
    [
      nodejs_24
      npmHooks.npmConfigHook
      cargo-tauri.hook
      pkg-config
    ]
    ++ lib.optionals stdenv.hostPlatform.isLinux [ wrapGAppsHook3 ]
    ++ lib.optionals stdenv.hostPlatform.isDarwin [ makeBinaryWrapper ];

  buildInputs =
    [
      openssl
      libiconv
    ]
    ++ lib.optionals stdenv.hostPlatform.isLinux [
      libayatana-appindicator
      glib-networking # Most Tauri apps need networking
      libsoup_3
      webkitgtk_4_1
    ];

  doCheck = false;

  preFixup = lib.optionalString stdenv.hostPlatform.isLinux ''
    gappsWrapperArgs+=(--prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath [ libayatana-appindicator ]}")
  '';

  postInstall = lib.optionalString stdenv.hostPlatform.isDarwin ''
    makeWrapper $out/Applications/wireguard-gui.app/Contents/MacOS/wireguard-gui $out/bin/wireguard-gui
  '';

  env = {
    OPENSSL_NO_VENDOR = true;
    # NODE_ENV = "production";
  };
}
