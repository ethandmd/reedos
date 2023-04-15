
spin.elf: spin.o
	riscv64-unknown-elf-ld -T spin.ld spin.o -o spin.elf -no-pie --entry=entry

spin.o: spin.s
	riscv64-unknown-elf-as spin.s -o spin.o

