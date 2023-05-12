#! /usr/bin/env bash

##
## This is just a wrapper for QEMU that adds the -s -S (for gdb) flags when
## DEBUG is set, to be used as a binary runner by Cargo.
##

set -euo pipefail

## Creates a 'raw' ext2 fs on fs.img file.
##
## dd if=/dev/zero of=fs.img bs=512 count=20480
##
## Specify 4k block size, check fs.img for bad blocks, force direct IO, and optionally
## copy /usr/bin (e.g. builtin programs) to the filesystem.
## mkfs.ext2 fs.img -b 4096 -c fs.img -D # -d /usr/bin

# ** Don't forget to `$qemu-img create fs.img 64k` (or whatever size you want).
FLAGS=(-machine virt -smp 2 -m 128M -bios none -nographic \
    -global virtio-mmio.force-legacy=false \
    -drive file=fs.img,if=none,format=raw,id=x0,read-only=off \
    -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0)

print_help() { echo "$(tput setaf 2)$(tput bold)(info)$(tput sgr0) $1"; }

print_help "Type CTRL-A, X to exit QEMU"
if [ -n "${DEBUG+x}" ] ; then
    FLAGS+=(-s -S)
    print_help "Starting QEMU in debug mode (connect with gdb)"
fi
set -x
exec "qemu-system-$1" "${FLAGS[@]}" -kernel "$2"
