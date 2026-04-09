{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
    strictDeps = true;

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
      gtk4
      libsoup_3
      glib
      pango
      gdk-pixbuf
    ];
    LIBRARY_PATH = "${pkgs.gdk-pixbuf}/lib";
}
