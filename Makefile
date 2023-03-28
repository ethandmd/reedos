RISCV64_AS ?= riscv64-none-elf-gcc-as
RISCV64_LD ?= riscv64-none-elf-gcc-ld
QEMU_FLAGS ?= -machine virt -smp 2 -m 128M -bios none -nographic

LIBREEDOS := target/riscv64imac-unknown-none-elf/debug/libreedos.a
ASM_FILES := $(shell find src -name *.s)
OBJ_FILES := $(ASM_FILES:.s=.o) $(LIBREEDOS)

################################################################

ifeq ($(DEBUG),1)
default: qemu-gdb
else
default: qemu
endif

################################################################

%.o: %.s
	$(RISCV64_AS) -c $^ -o $@ -march=rv64imac_zicsr_zifencei

$(LIBREEDOS): .ALWAYS
	cargo build

reedos.elf: kernel.ld $(OBJ_FILES)
	$(RISCV64_LD) -T$< -o $@ $(filter-out $<,$^)

################################################################

qemu-gdb: reedos.elf
	$(info $(shell tput setaf 5)Use CTRL-A, X to quit QEMU.$(shell tput sgr0))
	qemu-system-riscv64 -s -S $(QEMU_FLAGS) -kernel $<

qemu: reedos.elf
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
	rm -rf src/asm/*.o
	rm -rf reedos.elf

.PHONY: .ALWAYS
