        ## This program is for testing reedos
        ##
        ## It should just yield over and over. Test with prints in the
        ## rust handlers

        .global entry
entry:
spin:
        li a7, 124                #yield
        scall
        j spin
