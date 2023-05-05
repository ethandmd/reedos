        ## This is a quick test of the process write syscall, with the
        ## assumption that currently every process's stdout is
        ## hardwired to uart out

        .global entry
entry:
        li a7, 64                #write
        li a0, 1                 #stdout
        la a1, msg1
        li a2, 36               #msg length
        scall

end:
        li a7, 64                #write
        li a0, 1                 #stdout
        la a1, msg2
        li a2, 30               #msg length
        scall
spin:
        j spin

msg1:
        .ascii "Mission Control, This is a process.\n"
msg2:
        .ascii "Test Mission over. Spinning.\n"
