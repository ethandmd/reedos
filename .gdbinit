# Enable .gdbinit config by setting adding below to ~/.config/gdb/gdbinit:
# add-auto-load-safe-path $(pwd)/.gdbinit
target extended-remote localhost:1234
add-inferior
info threads
set schedule-multiple on
