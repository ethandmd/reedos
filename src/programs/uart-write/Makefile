
uart-write.elf: uart-write.o
	riscv64-unknown-elf-ld uart-write.o -o uart-write.elf -no-pie --entry=entry

uart-write.o: uart-write.s
	riscv64-unknown-elf-as uart-write.s -o uart-write.o

