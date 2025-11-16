{
  lib,
  stdenv,
  rustPlatform,
  fetchNpmDeps,
  cargo-tauri_1,
  darwin,
  glib-networking,
  libsoup_3,
  nodejs_22,
  npmHooks,
  libiconv,
  openssl,
  pkg-config,
  webkitgtk,
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
  inherit (toml) version;

  npmDeps = fetchNpmDeps {
    inherit pname version src;
    hash = "sha256-5nteidw81/EUc3TJT3WFC+m3pN/ZuUeepYKdvn/VwzA=";
  };

  cargoRoot = "src-tauri";
  useFetchCargoVendor = true;

  cargoLock.lockFile = "${src}/src-tauri/Cargo.lock";

  buildAndTestSubdir = cargoRoot;

  nativeBuildInputs =
    [
      nodejs_22
      npmHooks.npmConfigHook
      cargo-tauri_1.hook
      pkg-config
    ]
    ++ lib.optionals stdenv.hostPlatform.isLinux [ wrapGAppsHook3 ]
    ++ lib.optionals stdenv.hostPlatform.isDarwin [ makeBinaryWrapper ];

  buildInputs =
    [
      openssl
      libiconv
    ]
    ++ lib.optionals stdenv.isLinux [
      glib-networking # Most Tauri apps need networking
      libsoup
      webkitgtk
    ]
    ++ lib.optionals stdenv.isDarwin (
      with darwin.apple_sdk.frameworks;
      [
        AppKit
        CoreServices
        Security
        WebKit
      ]
    );

  doCheck = false;

  postInstall = lib.optionalString stdenv.hostPlatform.isDarwin ''
    makeWrapper $out/Applications/wireguard-gui.app/Contents/MacOS/wireguard-gui $out/bin/wireguard-gui
  '';

  env = {
    OPENSSL_NO_VENDOR = true;
    # NODE_ENV = "production";
  };
}
