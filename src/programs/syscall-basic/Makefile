
syscall-basic.elf: syscall-basic.o
	riscv64-unknown-elf-ld syscall-basic.o -o syscall-basic.elf -no-pie --entry=entry

syscall-basic.o: syscall-basic.s
	riscv64-unknown-elf-as syscall-basic.s -o syscall-basic.o

