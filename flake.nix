{
  description = "devcontainer-env - direnv that bridges devcontainers and the host environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = manifest.name;
          inherit (manifest) version;
          cargoLock.lockFile = ./Cargo.lock;
          src = pkgs.lib.cleanSource ./.;
          doCheck = false;
          meta = with pkgs.lib; {
            description = "direnv that bridges devcontainers and the host environment";
            homepage = "https://github.com/devcontainer-env/devcontainer-env";
            license = licenses.mit;
            mainProgram = manifest.name;
          };
        };

        devShells.default = pkgs.mkShell {
          name = "devcontainer-env";
          packages = with pkgs; [
            cargo
            clippy
            rustc
            rustfmt
            rust-analyzer
          ];
        };
      }
    );
}
