{ config, lib, pkgs, ... }:

let
  cfg = config.services.ddns-monitor;
  tomlFormat = pkgs.formats.toml { };

  # Map subscriber configs to TOML representation
  subscribersConfig = map (sub: {
    inherit (sub) provider domain;
    credentials_file =
      if sub.credentialsFile != null
      then "/run/credentials/ddns-monitor.service/${sub.domain}.secret"
      else null;
    credentials_env = sub.credentialsEnv;
  }) cfg.subscribers;

  # Build the declarative TOML configuration file
  configFile = tomlFormat.generate "ddns-monitor-config.toml" {
    ip_providers = {
      urls = cfg.ipProviders.urls;
      timeout_secs = cfg.ipProviders.timeoutSecs;
    };
    subscribers = subscribersConfig;
  };

  # Extract LoadCredential strings for systemd
  credentialLoads = lib.concatMap (sub:
    lib.optional (sub.credentialsFile != null)
      "${sub.domain}.secret:${sub.credentialsFile}"
  ) cfg.subscribers;

in
{
  options.services.ddns-monitor = {
    enable = lib.mkEnableOption "ddns-monitor WAN IP change detector and DDNS updater";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.ddns-monitor or (pkgs.rustPlatform.buildRustPackage {
        pname = "ddns-monitor";
        version = "0.1.0";
        src = ../.;
        cargoLock = {
          lockFile = ../Cargo.lock;
        };
        meta = with lib; {
          description = "Provider-agnostic DDNS monitor with plugin architecture";
          license = licenses.unlicense;
        };
      });
      description = "The ddns-monitor package to use.";
    };

    interval = lib.mkOption {
      type = lib.types.str;
      default = "30min";
      description = "Interval between DDNS monitor executions.";
    };

    onBootSec = lib.mkOption {
      type = lib.types.str;
      default = "5min";
      description = "Delay after system boot before the first execution.";
    };

    ipProviders = {
      urls = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [
          "https://api.ipify.org"
          "https://ifconfig.me/ip"
          "https://icanhazip.com"
        ];
        description = "List of external IP discovery echo endpoints.";
      };

      timeoutSecs = lib.mkOption {
        type = lib.types.int;
        default = 5;
        description = "Timeout in seconds for external IP discovery requests.";
      };
    };

    subscribers = lib.mkOption {
      type = lib.types.listOf (lib.types.submodule {
        options = {
          provider = lib.mkOption {
            type = lib.types.str;
            default = "dynu";
            description = "DDNS provider plugin name (e.g. 'dynu').";
          };

          domain = lib.mkOption {
            type = lib.types.str;
            description = "Domain name to update with the new WAN IP.";
          };

          credentialsFile = lib.mkOption {
            type = lib.types.nullOr lib.types.path;
            default = null;
            description = "Path to the decrypted secret or sops credential file.";
          };

          credentialsEnv = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
            description = "Environment variable name containing the credential.";
          };
        };
      });
      default = [ ];
      description = "List of dynamic DNS subscribers to notify upon IP change.";
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.ddns-monitor = {
      description = "DDNS Smart IP Change Monitor and Updater";
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];

      serviceConfig = {
        Type = "oneshot";
        User = "root";
        Group = "root";
        StateDirectory = "ddns-monitor";
        ExecStart = "${cfg.package}/bin/ddns-monitor --config ${configFile}";
        TimeoutSec = 60;
        LoadCredential = credentialLoads;

        # Hardening
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ "/var/lib/ddns-monitor" ];
      };
    };

    systemd.timers.ddns-monitor = {
      description = "Run ddns-monitor periodically";
      timerConfig = {
        OnBootSec = cfg.onBootSec;
        OnUnitActiveSec = cfg.interval;
        AccuracySec = "1min";
        Persistent = true;
      };
      wantedBy = [ "timers.target" ];
    };
  };
}
