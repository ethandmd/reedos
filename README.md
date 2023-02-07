## Setup
- qemu for riscv
- riscv-gnu-toolchain (and add to PATH)
- rustup add target riscv64gc-unkown-none-elf

## Usage
`$ make run # "Ctrl-a" + "x" to quit qemu`

On this branch, expect to see:
```
$ make run
cargo build
warning: file `/home/ethan/Documents/reed/y3s2/393/reedos/src/main.rs` found to be present in multiple build targets:
  * `lib` target `reedos`
  * `bin` target `reedos`
   Compiling reedos v0.1.0 (/home/ethan/Documents/reed/y3s2/393/reedos)
    Finished dev [unoptimized + debuginfo] target(s) in 0.26s
riscv64-unknown-elf-ld -Tkernel.ld target/riscv64gc-unknown-none-elf/debug/libreedos.a -o reedos.ELF
echo "Ctrl-a x to quit qemu"
Ctrl-a x to quit qemu
qemu-system-riscv64 \
	-machine virt \
	-m 2G \
	-bios none \
	-nographic \
	-kernel reedos.ELF
[INFO]: Currently on hartid: 0
[INFO]: main fn's addr?: 0x800001c4
[INFO]: Jumping to main fn (and sup mode)
[INFO]: Entered main()
MELLOW SWIRLED!
 from,
 your fav main fn
(called from _start fn!)
QEMU: Terminated
```


### Notes
compile the asm with risc*gcc -c to an elf object file

compile the rust with cargo build using staticlib

use risc*ld -Tkernel.ld to link entry.o (step 1) with libreedos.a (step 2) and output as elf executable

run with qemu-system-riscv64 -m 2G -machine virt -bios none -kernel [elf exe output]

^ we *need* to set -m 2G to expand the memory, as currently the stack is being set to 0x9000_0000

### References
+ https://osblog.stephenmarz.com/index.html
+ https://github.com/mit-pdos/xv6-riscv
