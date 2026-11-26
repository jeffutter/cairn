# Home Manager module: `cairn serve` as a launchd agent (macOS).
#
# The server listens on free ports chosen at start. The CLI finds it over the
# control socket beside the database (`cairn status`, `cairn view`).
self:
{ config, lib, pkgs, ... }:
let
  cfg = config.services.cairn;
  logs = "${config.home.homeDirectory}/Library/Logs";
in
{
  options.services.cairn = {
    enable = lib.mkEnableOption "the cairn review server as a launchd agent";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      defaultText = lib.literalExpression "cairn.packages.\${system}.default";
      description = "The cairn package to run. Install the same one for the CLI.";
    };

    extraArgs = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "--bind" "0.0.0.0" "--advertise-host" "mac.lan" ];
      description = ''
        Further `cairn serve` flags. A non-loopback `--bind` has no
        authentication: anyone who can reach the ports can read and comment.
        Do not pass `--port`; the module already does.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = pkgs.stdenv.hostPlatform.isDarwin;
        message = "services.cairn runs under launchd and supports only macOS";
      }
    ];

    launchd.agents.cairn = {
      enable = true;
      config = {
        ProgramArguments = [ "${cfg.package}/bin/cairn" "serve" "--port" "0" ] ++ cfg.extraArgs;
        RunAtLoad = true;
        KeepAlive = true;
        # launchd does not pass the shell's XDG_DATA_HOME, and the server and
        # CLI find each other through the database path.
        EnvironmentVariables.CAIRN_DB = "${config.xdg.dataHome}/cairn/cairn.db";
        StandardOutPath = "${logs}/cairn.log";
        StandardErrorPath = "${logs}/cairn.err.log";
      };
    };
  };
}
