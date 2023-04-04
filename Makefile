default: qemu

build: .ALWAYS
	cargo build

qemu: .ALWAYS
	cargo run

qemu-gdb:
	DEBUG=1 cargo run

lint: .ALWAYS
	cargo fmt --all -- --check
	cargo clippy

docs: .ALWAYS
	cargo doc --open

clean: .ALWAYS
	cargo clean

.PHONY: default .ALWAYS
