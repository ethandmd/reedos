LIBREEDOS=target/riscv64gc-unknown-none-elf/debug/libreedos.a

build:
	cargo build
	riscv64-unknown-elf-ld -Tkernel.ld $(LIBREEDOS) -o reedos.ELF

lint: 
	rustup component add rustfmt # Not for nightly
	cargo fmt --all -- --check #Add config
	cargo clippy
	# cargo doc <-- should start documenting

run: build
	echo "Ctrl-a x to quit qemu"
	qemu-system-riscv64 \
		-machine virt \
		-smp 2 \
		-m 2G \
		-bios none \
		-nographic \
		-kernel reedos.ELF

clean:
	cargo clean
	rm -rf src/*.o
	rm -rf reedos.ELF
