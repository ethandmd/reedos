{
  description = "Tools needed to develop reedos";

  inputs.rust-overlay = {
    url = "github:oxalica/rust-overlay";
    inputs.nixpkgs.follows = "nixpkgs";
    inputs.flake-utils.follows = "flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        riscv64-cc = pkgs.pkgsCross.riscv64-embedded.stdenv.cc;
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            (rust-bin.nightly.latest.default.override {
              extensions = [ "rust-src" ];
              targets = [ "riscv64imac-unknown-none-elf" ];
            })
            riscv64-cc
            qemu
          ];

          RISCV64_AS = "${riscv64-cc}/bin/${riscv64-cc.targetPrefix}as";
          RISCV64_LD = "${riscv64-cc}/bin/${riscv64-cc.targetPrefix}ld";
        };
      });
}
