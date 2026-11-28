# Home Manager module: `cairn serve` as a launchd agent (macOS).
#
# The server listens on `services.cairn.port` (default 7878; 0 picks free
# ports). The CLI finds it over the control socket beside the database
# (`cairn status`, `cairn view`).
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

    port = lib.mkOption {
      type = lib.types.port;
      default = 7878;
      example = 0;
      description = ''
        Web UI port. The content port is the next one up, unless `extraArgs`
        sets `--content-port`. 0 picks free ports for both.
      '';
    };

    extraArgs = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "--bind" "0.0.0.0" "--advertise-host" "mac.lan" ];
      description = ''
        Further `cairn serve` flags. A non-loopback `--bind` has no
        authentication: anyone who can reach the ports can read and comment.
        Do not pass `--port`; set the `port` option.
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
        ProgramArguments = [ "${cfg.package}/bin/cairn" "serve" "--port" (toString cfg.port) ] ++ cfg.extraArgs;
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
