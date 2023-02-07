# reedos
You could say reed-oh-ESS, but we like to think it's barely FreeDOS minus the 'f'. Like rEE-doss. 

## Setup
In order to get started with this project you'll need the following:
- Rust (No guarantees this compiles <= 1.67, works on stable at the moment.)
- `riscv-gnu-toolchain` (don't forget to add to PATH)
- `rustup add target riscv64gc-unkown-none-elf`
## Usage
- Build, link, and run reedos on Qemu virt machine:
`$ make run`
- Clean up build artifacts:
`$ make clean`

### Debug tools
You may find the following debug tools (that you have mostly already installed) helpful:
- `riscv64-unknown-elf-{nm, objcopy, objdump, gdb, gcov*}`
- In Qemu with `-nographic` use `Ctr+A` then `c` to get to console, run `help` to see available commands.

## Organization
```
├── .cargo
│   └── config.toml // Compile and linking setups
├── Cargo.lock
├── Cargo.toml
├── kernel.ld // Linker script
├── Makefile
├── README.md
├── rust-toolchain // Specifies stable toolchain
└── src
    ├── entry.rs // Setup a stack and jump to _start() in main.rs
    ├── log.rs // Logging macros (and currently home to print macros)
    ├── main.rs // _start() set up routine which calls to main()
    ├── param.rs // Global system parameters (memlayout, etc)
    ├── riscv.rs // Wrappers around unsafe asm calls
    ├── timervec.rs // Configure and handle interrupts
    └── uart.rs // Marz's MMIO uart driver code (our refactor to follow)
```

### References
+ https://five-embeddev.com/riscv-isa-manual/latest/csr.html
+ https://github.com/mit-pdos/xv6-riscv
+ https://osblog.stephenmarz.com/index.html
+ https://os.phil-opp.com/
