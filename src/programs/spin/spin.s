        ## This program is for testing reedos
        ##
        ## It should just spin in place

        .global entry
entry:
        li x0, 0x0badcafe
spin:
        j spin
