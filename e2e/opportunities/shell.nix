# NixOS / nix-shell entry for Opportunities Playwright tests.
# Usage from repo root: nix-shell e2e/opportunities/shell.nix --run ./scripts/run-opportunities-e2e.sh
{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    nodejs_22
    playwright-driver.browsers
    firefox
  ];

  shellHook = ''
    export PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1
    export JOBDESK_E2E_FIREFOX="${pkgs.playwright-driver.browsers}/firefox-1495/firefox/firefox"
    export JOBDESK_E2E_CHROMIUM="${pkgs.playwright-driver.browsers}/chromium-1194/chrome-linux/chrome"
    unset PLAYWRIGHT_BROWSERS_PATH
    unset NIX_PLAYWRIGHT_BROWSERS
  '';
}
