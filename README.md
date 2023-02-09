# reedos
You could say reed-oh-ESS, but we like to think it's barely FreeDOS minus the 'f'. Like rEE-doss. 

## Setup
In order to get started with this project you'll need the following:
- Rust (No guarantees this compiles <= 1.67, works on stable at the moment.)
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
+ https://riscv.org/technical/specifications/ (Go to "Volume 2, Privileged Specification")
+ https://five-embeddev.com/riscv-isa-manual/latest/csr.html
+ https://github.com/mit-pdos/xv6-riscv
+ https://osblog.stephenmarz.com/index.html
+ https://os.phil-opp.com/
