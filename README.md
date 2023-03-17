# reedos
You could say reed-oh-ESS, but we like to think it's barely FreeDOS minus the 'f'. Like rEE-doss. 

See [Contribution Guidelines](CONTRIBUTING.md) if you're interested in getting involved.

### Notes
We currently support Rust's `GlobalAlloc` in order to use the `alloc` crate. We do so by wrapping page 
allocation and finer grained virtual memory allocation into a `Global Allocator` struct which implements
Rust's `GlobalAlloc` trait.

## Setup
In order to get started with this project you'll need the following:
- Rust (currently on nightly branch) 
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
- `riscv64-unknown-elf-{objdump, gdb}` <-- Strongly recommend viewing docs material on becoming a GDB power user.
- In Qemu with `-nographic` use `Ctr+A` then `c` to get to console, run `help` to see available commands.

### Docs
 + `$ make docs` builds and open documentation in browser (equal to `cargo docs --open`, can't ever remember this command).
 
### References
+ [ISA Manual](https://riscv.org/technical/specifications/) (Go to "Volume 2, Privileged Specification")
+ [Nomicon](https://doc.rust-lang.org/nomicon/)
+ [Interrupt Cookbook](https://www.starfivetech.com/uploads/sifive-interrupt-cookbook-v1p2.pdf)
+ [MIT's XV6-RISCV](https://github.com/mit-pdos/xv6-riscv)
+ [Marz's OSDEV Blog OS](https://osblog.stephenmarz.com/index.html)
+ [Phil-Opp's Blog OS](https://os.phil-opp.com/)
