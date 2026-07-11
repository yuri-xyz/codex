{
  cmake,
  llvmPackages,
  openssl,
  fetchurl,
  libcap ? null,
  craneLib,
  pkg-config,
  lib,
  stdenv,
  version ? "0.0.0",
  ...
}:
let
  commonArgs = {
    env = {
      PKG_CONFIG_PATH = lib.makeSearchPathOutput "dev" "lib/pkgconfig" (
        [ openssl ] ++ lib.optionals stdenv.isLinux [ libcap ]
      );
      CARGO_PROFILE_RELEASE_LTO = "false";
      RUSTY_V8_ARCHIVE = fetchurl {
        name = "librusty_v8-146.4.0";
        url = "https://github.com/denoland/rusty_v8/releases/download/v146.4.0/librusty_v8_release_${stdenv.hostPlatform.rust.rustcTarget}.a.gz";
        hash = "sha256-5ktNmeSuKTouhGJEqJuAF4uhA4LBP7WRwfppaPUpEVM=";
      };
    };
    pname = "codex-rs";
    inherit version;
    cargoExtraArgs = "--locked -p codex-cli --bin codex";
    doCheck = false;
    src = craneLib.cleanCargoSource ./.;

    # Patch the workspace Cargo.toml so that cargo embeds the correct version in
    # CARGO_PKG_VERSION (which the binary reads via env!("CARGO_PKG_VERSION")).
    # On release commits the Cargo.toml already contains the real version and
    # this sed is a no-op.
    postPatch = ''
      sed -i 's/^version = "0\.0\.0"$/version = "${version}"/' Cargo.toml
    '';
    nativeBuildInputs = [
      cmake
      llvmPackages.clang
      llvmPackages.libclang.lib
      openssl
      pkg-config
    ] ++ lib.optionals stdenv.isLinux [
      libcap
    ];

    cargoLock = ./Cargo.lock;
    outputHashes = {
      "git+https://github.com/dzbarsky/rules_rust?rev=b56cbaa8465e74127f1ea216f813cd377295ad81#b56cbaa8465e74127f1ea216f813cd377295ad81" = "sha256-uJpVLcQh8wWZA3GPv9D8Nt43EOirajfDJ7eq/FB+tek=";
      "git+https://github.com/helix-editor/nucleo.git?rev=4253de9faabb4e5c6d81d946a5e35a90f87347ee#4253de9faabb4e5c6d81d946a5e35a90f87347ee" = "sha256-Hm4SxtTSBrcWpXrtSqeO0TACbUxq3gizg1zD/6Yw/sI=";
      "git+https://github.com/juberti-oai/rust-sdks.git?rev=e2d1d1d230c6fc9df171ccb181423f957bb3c1f0#e2d1d1d230c6fc9df171ccb181423f957bb3c1f0" = "sha256-gkXY6kr9ETnRC62TkKgKhqvxPTdACEWaTbtNALhRvAM=";
      "git+https://github.com/nornagon/crossterm?rev=87db8bfa6dc99427fd3b071681b07fc31c6ce995#87db8bfa6dc99427fd3b071681b07fc31c6ce995" = "sha256-6qCtfSMuXACKFb9ATID39XyFDIEMFDmbx6SSmNe+728=";
      "git+https://github.com/nornagon/ratatui?rev=9b2ad1298408c45918ee9f8241a6f95498cdbed2#9b2ad1298408c45918ee9f8241a6f95498cdbed2" = "sha256-HBvT5c8GsiCxMffNjJGLmHnvG77A6cqEL+1ARurBXho=";
      "git+https://github.com/openai-oss-forks/tokio-tungstenite?rev=0e5b2d73aa18dd9f0a50ee9ff199d5aef7594186#0e5b2d73aa18dd9f0a50ee9ff199d5aef7594186" = "sha256-V1xmnrfRWOcZZogelZEA4vvyMj2awCfHVA5/glQ6KAI=";
      "git+https://github.com/openai-oss-forks/tungstenite-rs?rev=4fffad30fe373adbdcffab9545e9e9bf4f2fc19f#4fffad30fe373adbdcffab9545e9e9bf4f2fc19f" = "sha256-VVHhk7l9J/sEmG3q/UuV/sQ3f+fGsmq5vumSy8vbMvw=";
    };
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
craneLib.buildPackage (commonArgs // {
  inherit cargoArtifacts;

  meta = with lib; {
    description = "OpenAI Codex command‑line interface rust implementation";
    license = licenses.asl20;
    homepage = "https://github.com/openai/codex";
    mainProgram = "codex";
  };
})
