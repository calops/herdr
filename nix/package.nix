{
  lib,
  stdenv,
  rustPlatform,
  callPackage,
  runCommand,
  zig_0_16,
  zstd,
  pkg-config,
  git,
  just,
  libnotify,
  installShellFiles,
  cctools ? null,
  xcbuild ? null,
}:

let
  manifest = lib.importTOML ../Cargo.toml;
  zigDeps = callPackage ../vendor/libghostty-vt/build.zig.zon.nix {
    name = "herdr-libghostty-vt-zig-cache";
    inherit zstd;
    linkFarm =
      name: entries:
      runCommand name { } ''
        mkdir -p $out
        ${lib.concatMapStringsSep "\n" (entry: ''
          cp -rL ${entry.path} $out/${entry.name}
        '') entries}
      '';
  };
  darwinToolchain = lib.optionals stdenv.hostPlatform.isDarwin [
    cctools
    xcbuild
  ];
in
rustPlatform.buildRustPackage {
  pname = "herdr";
  version = manifest.package.version;

  src = lib.fileset.toSource {
    root = ./..;
    fileset = lib.fileset.intersection (lib.fileset.fromSource (lib.sources.cleanSource ./..)) (
      lib.fileset.unions [
        ../assets
        ../crates
        ../distribution/install.ps1
        ../distribution/latest.json
        ../docs/next/api/herdr-api.schema.json
        ../src
        ../tests/fixtures
        ../vendor/libghostty-vt
        ../vendor/libghostty-vt.vendor.json
        ../vendor/portable-pty
        ../build.rs
        ../Cargo.lock
        ../Cargo.toml
        ../skills/herdr/SKILL.md
        ../justfile
      ]
    );
  };

  cargoLock = {
    lockFileContents = builtins.readFile ../Cargo.lock;
  };

  nativeBuildInputs = [
    git
    pkg-config
    installShellFiles
  ] ++ darwinToolchain;

  nativeCheckInputs = [ just ];

  postPatch = ''
    substituteInPlace crates/ghostty-vt/build.rs \
      --replace-fail '.arg("build")' '.arg("build")
          .arg("-Dcpu=baseline")' \
      --replace-fail '.arg(format!("-Dtarget={zig_target}"))' ""
  '' + lib.optionalString stdenv.hostPlatform.isLinux ''
    substituteInPlace src/platform/linux.rs \
      --replace-fail 'let mut cmd = command("notify-send");' \
        'let mut cmd = command("${libnotify}/bin/notify-send");'
  '';

  env = {
    LIBGHOSTTY_VT_OPTIMIZE = "ReleaseFast";
    LIBGHOSTTY_VT_SIMD = "true";
    LIBGHOSTTY_VT_ZIG_SYSTEM_DIR = zigDeps;
    ZIG = lib.getExe zig_0_16;
    CARGO_BUILD_TARGET = stdenv.hostPlatform.rust.rustcTarget;
  };

  preBuild = ''
    export ZIG_GLOBAL_CACHE_DIR="$TMPDIR/zig-global-cache"
    export ZIG_LOCAL_CACHE_DIR="$TMPDIR/zig-local-cache"
  '';

  doCheck = stdenv.buildPlatform.canExecute stdenv.hostPlatform;
  checkPhase = ''
    runHook preCheck
    just nix-test
    just bench-render-scale server::render_scale_benchmark::render_scale_profile
    runHook postCheck
  '';

  postInstall = lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
    installShellCompletion --cmd herdr \
      --bash <("$out/bin/herdr" completion bash) \
      --fish <("$out/bin/herdr" completion fish) \
      --zsh <("$out/bin/herdr" completion zsh)
  '';

  meta = {
    description = "Terminal workspace manager for AI coding agents";
    homepage = "https://herdr.dev";
    license = lib.licenses.asl20;
    mainProgram = "herdr";
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
