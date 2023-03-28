RISCV64_AS ?= riscv64-none-elf-gcc-as
RISCV64_LD ?= riscv64-none-elf-gcc-ld
QEMU_FLAGS ?= -machine virt -smp 2 -m 128M -bios none -nographic

REEDOS := target/riscv64imac-unknown-none-elf/debug/reedos

################################################################

ifeq ($(DEBUG),1)
default: qemu-gdb
else
default: qemu
endif

################################################################

$(REEDOS): .ALWAYS
	cargo build

qemu-gdb: $(REEDOS)
	$(info $(shell tput setaf 5)Use CTRL-A, X to quit QEMU.$(shell tput sgr0))
	qemu-system-riscv64 -s -S $(QEMU_FLAGS) -kernel $<

qemu: $(REEDOS)
	$(info $(shell tput setaf 5)Use CTRL-A, X to quit QEMU.$(shell tput sgr0))
	qemu-system-riscv64 $(QEMU_FLAGS) -kernel $<

################################################################

lint: .ALWAYS
	cargo fmt --all -- --check
	cargo clippy

docs: .ALWAYS
	cargo doc --open

clean: .ALWAYS
	cargo clean

.PHONY: .ALWAYS
