{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
    strictDeps = true;
    nativeBuildInputs = with pkgs; [
      cargo
      rustc
      pkg-config
      avahi
      avahi-compat
    ];
    buildInputs = with pkgs; [
      gtk4
      libsoup_3
      glib
      pango
      gdk-pixbuf
    ];
    LIBRARY_PATH = "${pkgs.gdk-pixbuf}/lib";
}
