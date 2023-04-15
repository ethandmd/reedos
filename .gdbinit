# Enable .gdbinit config by setting adding below to ~/.config/gdb/gdbinit:
# add-auto-load-safe-path $(pwd)/.gdbinit
target extended-remote localhost:1234
add-inferior
info threads
set schedule-multiple on
set output-radix 16
set print pretty on
set disassemble-next-line auto
# tui enable
define hook-quit
    set confirm off
end
define hook-kill
    set confirm off
end

