# args and whatnot
LIBREEDOS=target/riscv64gc-unknown-none-elf/debug/libreedos.a

ASM-GCC=riscv64-unknown-elf-gcc
ASM-CFLAGS=

LINKER=riscv64-unknown-elf-ld
# first target is default
ifeq ($(DEBUG),1)
run: gdb-start
else
run: start
endif


# make global rule for all asm files
%.o: %.s
	$(ASM-GCC) -c $^ -o $@

src/asm/entry.o: src/asm/entry.s
src/asm/trap.o: src/asm/trap.s

$(LIBREEDOS): .FORCE
	cargo build

# can't use automatic variables for dependencies, because we don't
# want to mix kernel.ld with the rest
# For reasons I don't understand, this order works and many others don't
build: kernel.ld $(LIBREEDOS) src/asm/entry.o src/asm/trap.o .FORCE
	$(LINKER) -Tkernel.ld -o reedos.ELF \
		src/asm/entry.o src/asm/trap.o $(LIBREEDOS)

# other nice stuff
lint:
	rustup component add rustfmt # Not for nightly
	cargo fmt --all -- --check #Add config
	cargo clippy

docs:
	cargo doc --open

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
	rm -rf src/asm/*.o
	rm -rf reedos.ELF

.PHONY: .FORCE
