{
  inputs = {
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk = {
      url = "github:nmattia/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, naersk }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlay ];
        };
        rust = pkgs.rust-bin.selectLatestNightlyWith (toolchain:
          toolchain.default.override { extensions = [ "rust-src" ]; });
        naersk-lib = naersk.lib."${system}".override { rustc = rust; };
      in {
        defaultPackage = naersk-lib.buildPackage {
          pname = "thread_master";
          root = ./.;
        };

        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            cargo-asm
            cargo-flamegraph
            gnuplot
            rust
          ];
        };
      }) // {
        overlay = final: prev: {
          thread_master = self.defaultPackage."${prev.system}";
        };

        nixosModule = { config, lib, pkgs, ... }:
          let cfg = config.services.thread_master;
          in {
            options.services.thread_master = {
              enable = lib.mkEnableOption "thread_master Discord Bot";

              package = lib.mkOption {
                type = lib.types.package;
                default = pkgs.thread_master;
                description = "User to run the service with";
              };

              user = lib.mkOption {
                type = lib.types.str;
                default = "thread_master";
                description = "User to run the service with";
              };

              tokenFile = lib.mkOption {
                type = lib.types.path;
                description = "Path to the token for a Discord bot";
              };

              channelIDs = lib.mkOption {
                type = lib.types.listOf lib.types.int;
                description = "Channel IDs to operate in";
              };
            };

            config = lib.mkMerge [
              ({ nixpkgs.overlays = [ self.overlay ]; })
              (lib.mkIf cfg.enable {
                users.users."${cfg.user}" = {
                  name = cfg.user;
                  isNormalUser = true;
                  description = "thread_master Discord Bot";
                  home = "/var/empty";
                  shell = null;
                };

                systemd.services.thread_master = {
                  description = "thread_master Discord Bot";
                  wantedBy = [ "multi-user.target" ];
                  environment.THREAD_CHANNEL_IDS = lib.concatMapStringsSep "," builtins.toString cfg.channelIDs;
                  serviceConfig = {
                    Type = "simple";
                    User = cfg.user;
                    Restart = "on-failure";
                    ExecStart = "${cfg.package}/bin/thread_master ${cfg.tokenFile}";
                  };
                };

              })
            ];
          };
      };
}
