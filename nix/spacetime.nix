# Spacetime, built by Nix — so `nix build` runs it on the configured remote
# builder (/etc/nix/machines; this workstation sets max-jobs = 0) instead of here.
#
#   nix build -f nix/spacetime.nix -L                       # release, cdp + defaults
#   nix build -f nix/spacetime.nix --arg features '[ ]'     # without cdp
#   ./result/bin/spacetime serve demos/film-robot-day/      # run from the repo root
#
# Dependencies are their own derivation (crane's buildDepsOnly), so after the
# first build only the spacetime crate itself recompiles on the builder.
#
# The source is the WORKING TREE (uncommitted edits included) minus what the
# binary never reads at build time: the vendored submodules (three.js, raea —
# their `*.bundle.js` artifacts stay, and build.rs skips rebundling when a
# bundle exists and bun is absent), scratch/, projects/, build outputs.
#
# rusty_v8 (rustyscript's V8) normally downloads its static library at build
# time; the sandbox has no network, so the archive is pinned here. Bump it with
# the `v8` crate version in Cargo.lock.
{ pkgs ? import (builtins.getFlake "nixpkgs") { }
, crane ? builtins.getFlake "github:ipetkov/crane"
, features ? [ "cdp" ]
}:
let
  lib = pkgs.lib;
  craneLib = crane.mkLib pkgs;
  root = ../.;

  skip = [
    "scratch" "projects" "target" "result" "dist" "node_modules" ".git"
    "stdlib/3d/vendor/three" "stdlib/film/vendor/raea"
  ];
  src = lib.cleanSourceWith {
    src = root;
    filter = path: type:
      let rel = lib.removePrefix (toString root + "/") (toString path);
      in !(lib.any (s: rel == s || lib.hasPrefix (s + "/") rel) skip)
         && !(lib.hasSuffix ".snap.new" rel);
  };

  v8 = pkgs.fetchurl {
    url = "https://github.com/denoland/rusty_v8/releases/download/v137.3.0/librusty_v8_release_x86_64-unknown-linux-gnu.a.gz";
    sha256 = "0rv5nl4gcbvdpk3mdwbqvw180nfx2wk173q1cqrvv2h1agg1ys52";
  };

  common = {
    inherit src;
    pname = "spacetime";
    version = "0.1.0";
    strictDeps = true;
    cargoExtraArgs = "--locked --bins" + lib.optionalString (features != [ ]) " --features ${lib.concatStringsSep "," features}";
    nativeBuildInputs = with pkgs; [ pkg-config ];
    buildInputs = with pkgs; [ openssl zlib ];
    RUSTY_V8_ARCHIVE = v8;
    doCheck = false;
  };
  depsFor = profile: craneLib.buildDepsOnly (common // {
    CARGO_PROFILE = profile;
    cargoCheckCommand = "true";
    # Cargo.toml patches serde/serde_core from .patches/ (path crates): the
    # deps build stubs every path crate, so it gets the REAL patched sources
    dummySrc = craneLib.mkDummySrc {
      inherit src;
      extraDummyScript = ''
        rm -rf $out/.patches
        cp -r ${src}/.patches $out/.patches
        chmod -R u+w $out/.patches
      '';
    };
  });
  deps = depsFor "release";
  spacetime = craneLib.buildPackage (common // {
    # deps only, built once and cached (only spacetime recompiles after)
    cargoArtifacts = deps;
    passthru = {
      # nix build -f nix/spacetime.nix tests      (cargo test --lib, on the builder)
      # the dev profile: the suite has debug-only helpers (cfg(debug_assertions))
      tests = craneLib.cargoTest (common // {
        CARGO_PROFILE = "dev";
        cargoArtifacts = depsFor "dev";
        doCheck = true;
        cargoTestExtraArgs = "--lib";
        RUST_TEST_THREADS = "1";
        RUST_MIN_STACK = "8388608";
      });
      # nix build -f nix/spacetime.nix headless   (the .test.st suite, by the built binary)
      headless = pkgs.runCommand "spacetime-headless-suite" { } ''
        cp -r ${src} work && chmod -R u+w work && cd work
        # the suite's own verdict is its summary line (it has known failures), not this build's
        ${spacetime}/bin/spacetime test tests/ --headless > $out 2>&1 || true
        tail -1 $out
      '';
    };
  });
in
spacetime
