        ## This program is for testing reedos
        ##
        ## It should attempt to call an unimplemented syscall, and
        ## thus panic in the appropriate handler

        .global entry
entry:
        ## use 1 or something to test unimplimented, use 124 to test yield
        li a7, 124                #try an unimplemented syscall
        scall
spin:
        j spin
