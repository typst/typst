#!/bin/sh
# Copyright 2019 the Deno authors. All rights reserved. MIT license.
# TODO(everyone): Keep this script simple and easily auditable.
# Forked from Deno's install.sh script

set -e

use_unzip=false

if [ "$OS" = "Windows_NT" ]; then
	target="x86_64-pc-windows-msvc"
	use_unzip=true
else
	case $(uname -sm) in
	"Darwin x86_64") target="x86_64-apple-darwin" ;;
	"Darwin arm64")
		echo "Error: Official Typst builds for Darwin arm64 are not available." 1>&2 # (see: https://github.com/denoland/deno/issues/1846 )" 1>&2
		exit 1
		;;
	"Linux aarch64")
		echo "Error: Official Typst builds for Linux aarch64 are not available." 1>&2 # (see: https://github.com/denoland/deno/issues/1846 )" 1>&2
		exit 1
		;;
	*) target="x86_64-unknown-linux-gnu" ;;
	esac
fi

if [ "$use_unzip" = true ] && ! command -v unzip >/dev/null; then
	echo "Error: unzip is required to install Typst." 1>&2 # (see: https://github.com/denoland/deno_install#unzip-is-required )." 1>&2
	exit 1
fi

typst_install="${TYPST_INSTALL:-$HOME/.typst}"
bin_dir="$typst_install/bin"
exe="$bin_dir/typst"

if [ ! -d "$bin_dir" ]; then
	mkdir -p "$bin_dir"
fi

if [ "$use_unzip" = true ]; then
	if [ $# -eq 0 ]; then
		typst_uri="https://github.com/typst/typst/releases/latest/download/typst-${target}.zip"
	else
		typst_uri="https://github.com/typst/typst/releases/download/${1}/typst-${target}.zip"
 	fi

	curl --fail --location --progress-bar --output "$exe.zip" "$typst_uri"
	unzip -d "$bin_dir" -o "$exe.zip"
	mv "$bin_dir/typst-${target}/*" "$bin_dir"
	chmod +x "$exe"
	rm -r "$bin_dir/typst-${target}/*"
	rm "$exe.zip"
else
	if [ $# -eq 0 ]; then
		typst_uri="https://github.com/typst/typst/releases/latest/download/typst-${target}.tar.gz"
	else
		typst_uri="https://github.com/typst/typst/releases/download/${1}/typst-${target}.tar.gz"
	fi

	curl --fail --location --progress-bar --output "$exe.tar.gz" "$typst_uri"
	tar -xf "$exe.tar.gz" -C "$bin_dir" --strip-components=1
	chmod +x "$exe"
	rm "$exe.tar.gz"
fi

echo "Typst was installed successfully to $exe"
if command -v typst >/dev/null; then
	echo "Run 'typst --help' to get started"
else
	case $SHELL in
	/bin/zsh) shell_profile=".zshrc" ;;
	*) shell_profile=".bashrc" ;;
	esac
	echo "Manually add the directory to your \$HOME/$shell_profile (or similar)"
	echo "  export TYPST_INSTALL=\"$typst_install\""
	echo "  export PATH=\"\$TYPST_INSTALL/bin:\$PATH\""
	echo "Run '$exe --help' to get started"
fi
echo
echo "Stuck? Join our Discord https://discord.gg/2uDybryKPe"
