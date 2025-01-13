{ lib, pkgs, modulesPath, system, cs2kz-api, ... }:

let
  sshKeys = [
    ''ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIB4SBKTQ7WJcihtw3QocLXi+xEc/6HklXigYoltI8iNH alphakeks@dawn''
    ''ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPe34iB4eZ5KnO8nKXHtH4V0QZNb7Ro/YxZw7xuCEJ7C max@framework''
  ];
in

{
  environment = {
    systemPackages = with pkgs; [ coreutils vim ];
    defaultPackages = with pkgs; [ tmux curl git btop fd fzf neovim jq ripgrep ];
    variables.EDITOR = "nvim";
  };
  networking = {
    hostName = "cs2kz-api";
    firewall = {
      interfaces = {
        "enp0s6" = {
          allowedTCPPorts = [ 22 80 443 ];
        };
      };
    };
  };
  nixpkgs.hostPlatform = system;
  programs.zsh.enable = true;
  security.acme = {
    acceptTerms = true;
    defaults.email = "cs2kz@dawn.sh";
  };
  services = {
    openssh = {
      enable = true;
      settings.PasswordAuthentication = false;
    };
    mysql = {
      enable = true;
      package = pkgs.mariadb;
      ensureDatabases = [ "cs2kz" ];
      ensureUsers = [{
        name = "schnose";
        ensurePermissions = {
          "cs2kz.*" = "ALL PRIVILEGES"; # TODO: more granular permissions
        };
      }];
      initialDatabases = [{
        name = "cs2kz";
        schema = ../crates/cs2kz/migrations/0001_initial.up.sql;
      }];
    };
    mysqlBackup = {
      enable = true;
      calendar = "02:30:00";
      databases = [ "cs2kz" ];
    };
    nginx = {
      enable = true;
      recommendedTlsSettings = true;
      recommendedProxySettings = true;
      virtualHosts."api.cs2kz.org" = {
        forceSSL = true;
        enableACME = true;
        locations."/" = {
          proxyPass = "http://[::1]:42069";
          proxyWebsockets = true;
          extraConfig = ''
            if ($cloudflare_ip != 1) {
              return 403;
            }

            # required when the server wants to use HTTP Authentication
            proxy_pass_header Authorization;
          '';
        };
      };
      commonHttpConfig =
        let
          realIpsFromList = lib.strings.concatMapStringsSep "\n" (x: "set_real_ip_from  ${x};");
          allowFromList = lib.strings.concatMapStringsSep "\n" (x: "${x} 1;");
          fileToList = x: lib.strings.splitString "\n" (builtins.readFile x);
          cfipv4 = fileToList (pkgs.fetchurl {
            url = "https://www.cloudflare.com/ips-v4";
            sha256 = "0ywy9sg7spafi3gm9q5wb59lbiq0swvf0q3iazl0maq1pj1nsb7h";
          });
          cfipv6 = fileToList (pkgs.fetchurl {
            url = "https://www.cloudflare.com/ips-v6";
            sha256 = "1ad09hijignj6zlqvdjxv7rjj8567z357zfavv201b9vx3ikk7cy";
          });
        in
        ''
          geo $realip_remote_addr $cloudflare_ip {
            default          0;
            ${allowFromList cfipv4}
            ${allowFromList cfipv6}
          }

          # Proxy CF-ConnectingIP header
          ${realIpsFromList cfipv4}
          ${realIpsFromList cfipv6}
          real_ip_header CF-Connecting-IP;
        '';
    };
  };
  system.stateVersion = "24.05";
  systemd.user.services.cs2kz-api = {
    enable = true;
    wantedBy = [ "multi-user.target" ];
    unitConfig.ConditionUser = "schnose";
    environment = {
      RUST_LOG = "cs2kz=trace,warn";
      KZ_API_ENVIRONMENT = "production";
    };
    script = ''
      ${cs2kz-api}/bin/cs2kz-api \
        --config "/etc/cs2kz-api.toml" \
        --depot-downloader-path "${pkgs.depotdownloader}/bin/DepotDownloader"
    '';
  };
  time.timeZone = "Europe/Berlin";
  users = {
    defaultUserShell = pkgs.zsh;
    users.root.openssh.authorizedKeys.keys = sshKeys;
    users.schnose = {
      isNormalUser = true;
      linger = true;
      useDefaultShell = true;
      extraGroups = [ "wheel" ];
      openssh.authorizedKeys.keys = sshKeys;
    };
  };
}
