LIBREEDOS=target/riscv64gc-unknown-none-elf/debug/libreedos.a

build:
	cargo build
	riscv64-unknown-elf-ld -Tkernel.ld $(LIBREEDOS) -o reedos.ELF

run: build
	echo "Ctrl-a x to quit qemu"
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
