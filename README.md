# reedos
You could say reed-oh-ESS, but we like to think it's barely FreeDOS minus the 'f'. Like rEE-doss.

See [Contribution Guidelines](CONTRIBUTING.md) if you're interested in getting involved.

## Setup
In order to get started with this project you'll need the following:

* Rust `rustc 1.69.0-nightly (c5c7d2b37 2023-02-24)` <-- the pointer arithmetic stuff will break.
* QEMU compiled for riscv
* `riscv-gnu-toolchain` (don't forget to add to PATH)
* `rustup target add riscv64gc-unkown-none-elf`

## Usage
- Build, link, and run reedos on Qemu virt machine:
`$ make run`
 - Qemu `-nographic` is exited with `C-a x`, that is `Control + a` followed by `x`.
   -  with `c` instead gives a console, but you will find `gdb` to much more helpful.
 - Clean up build artifacts:
`$ make clean`

## Tools and Resources
The following tools will prove useful. Guides on using them can be found in the root `docs` sub-directory. There you will also find links to other resources both about our project and things we have used in the creation of our project.

 - `riscv64-unknown-elf-{nm, objcopy, objdump, gdb, gcov*}`
 - `$ make docs` automatically builds and opens the documentation of this project in a browser.
   - `$ cargo docs` builds docs without opening them.

