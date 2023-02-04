LIBREEDOS=target/riscv64gc-unknown-none-elf/debug/libreedos.a

build:
	cargo build
	#cp target/riscv64gc-unknown-none-elf/debug/deps/reedos-*.o .
	riscv64-unknown-elf-gcc -c src/entry.S -o src/entry.o
	riscv64-unknown-elf-ld -Tkernel.ld src/entry.o $(LIBREEDOS) -o reedos.ELF

run: build
	qemu-system-riscv64 \
		-machine virt \
		-m 2G \
		-bios none \
		-nographic \
		-kernel reedos.ELF

clean:
	cargo clean
	rm -rf src/*.o
	rm -rf reedos.ELF
