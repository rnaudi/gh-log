{
  description = "GitHub PR analytics for your terminal.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "gh-log";
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          meta = with pkgs.lib; {
            description = "GitHub PR analytics for your terminal.";
            homepage = "https://github.com/rnaudi/gh-log";
            license = licenses.mit;
            maintainers = with maintainers; [ rnaudi ];
            mainProgram = "gh-log";
          };
        };

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/gh-log";
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
          ];
        };
      });
}
