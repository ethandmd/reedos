## Setup
- qemu for riscv
- riscv-gnu-toolchain (and add to PATH)
- rustup add target riscv64gc-unkown-none-elf

## Usage
`$ make run`


### Notes
compile the asm with risc*gcc -c to an elf object file

compile the rust with cargo build using staticlib

use risc*ld -Tkernel.ld to link entry.o (step 1) with libreedos.a (step 2) and output as elf executable

run with qemu-system-riscv64 -m 2G -machine virt -bios none -kernel [elf exe output]

^ we *need* to set -m 2G to expand the memory, as currently the stack is being set to 0x9000_0000

### References
+ https://osblog.stephenmarz.com/index.html
+ https://github.com/mit-pdos/xv6-riscv
