LIBREEDOS=target/riscv64gc-unknown-none-elf/debug/libreedos.a

build: kernel.ld .FORCE
	cargo build
	riscv64-unknown-elf-ld -Tkernel.ld $(LIBREEDOS) -o reedos.ELF

lint: 
	rustup component add rustfmt # Not for nightly
	cargo fmt --all -- --check #Add config
	cargo clippy

docs:
	cargo doc --open

ifeq ($(DEBUG),1)
run: gdb-start
else
run: start
endif

gdb-start: build
	echo "Ctrl-a x to quit qemu"
	qemu-system-riscv64 -s -S \
		-machine virt \
		-smp 2 \
		-m 128M \
		-bios none \
		-nographic \
		-kernel reedos.ELF

start: build
	echo "Ctrl-a x to quit qemu"
	qemu-system-riscv64 \
		-machine virt \
		-smp 2 \
		-m 128M \
		-bios none \
		-nographic \
		-kernel reedos.ELF

clean:
	cargo clean
	rm -rf src/*.o
	rm -rf reedos.ELF

.PHONY: .FORCE
