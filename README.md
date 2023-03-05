# reedos
You could say reed-oh-ESS, but we like to think it's barely FreeDOS minus the 'f'. Like rEE-doss. 

See [Contribution Guidelines](CONTRIBUTING.md) if you're interested in getting involved.

## Setup
In order to get started with this project you'll need the following:
- Rust `rustc 1.69.0-nightly (c5c7d2b37 2023-02-24)` <-- the pointer arithmetic stuff will break.
- QEMU compiled for riscv
- `riscv-gnu-toolchain` (don't forget to add to PATH)
- `rustup target add riscv64gc-unkown-none-elf`
## Usage
- Build, link, and run reedos on Qemu virt machine:
`$ make run`
- Clean up build artifacts:
`$ make clean`

### Debug tools
You may find the following debug tools (that you have mostly already installed) helpful:
- `riscv64-unknown-elf-{nm, objcopy, objdump, gdb, gcov*}`
- In Qemu with `-nographic` use `Ctr+A` then `c` to get to console, run `help` to see available commands.

### Docs
 + `$ make docs` automatically builds and open documentation in browser.
 + `$ cargo docs` build docs.

### References
+ [ISA Manual](https://riscv.org/technical/specifications/) (Go to "Volume 2, Privileged Specification")
+ [Interrupt Cookbook](https://www.starfivetech.com/uploads/sifive-interrupt-cookbook-v1p2.pdf)
+ [MIT's XV6-RISCV](https://github.com/mit-pdos/xv6-riscv)
+ [Marz's OSDEV Blog OS](https://osblog.stephenmarz.com/index.html)
+ [Phil-Opp's Blog OS](https://os.phil-opp.com/)
