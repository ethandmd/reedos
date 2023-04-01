#! /usr/bin/env bash

##
## This is just a wrapper for QEMU that adds the -s -S (for gdb) flags when
## DEBUG is set, to be used as a binary runner by Cargo.
##

set -euo pipefail

FLAGS=(-machine virt -smp 2 -m 128M -bios none -nographic)

print_help() { echo "$(tput setaf 2)$(tput bold)(info)$(tput sgr0) $1"; }

print_help "Type CTRL-A, X to exit QEMU"
if [ -n "${DEBUG+x}" ] ; then
    FLAGS+=(-s -S)
    print_help "Starting QEMU in debug mode (connect with gdb)"
fi
set -x
exec "qemu-system-$1" "${FLAGS[@]}" -kernel "$2"
