{ pkgs ? import <nixpkgs> {} }:

let
  dbus-dev = pkgs.dbus.dev;
in
pkgs.mkShell {
    nativeBuildInputs = with pkgs; [
      cargo
      rustc
      pkg-config
      avahi
      avahi-compat
      rust-analyzer
    ];
    buildInputs = with pkgs; [
      rust-analyzer
      dbus
      gtk4
      libsoup_3
      glib
      pango
      gdk-pixbuf
      mbedtls
    ];
    LIBRARY_PATH = "${pkgs.gdk-pixbuf}/lib:${pkgs.mbedtls}/lib";
    PKG_CONFIG_PATH = "${dbus-dev}/lib/pkgconfig";
}
