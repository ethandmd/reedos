```
┌──────────────────────┐ Top of physical memory.
│                      │
│                      │
│                      │
│                      │ Kernel Heap.
│                      │
│                      │
├──────────────────────┤
│      .bss            │
├──────────────────────┤
│      guard           │
│      H0 M-mode stack │
│      guard           │
│      H0 S-mode stack │
│      guard           │
│      H1 M-mode stack │
│      guard           │
│      H1 S-mode stack │
├──────────────────────┤
│      Hart 0 guard    │
│      Hart 0 stack    │
│                      │
├──────────────────────┤
│      Hart 1 guard    │
│      Hart 1 stack    │
│                      │
│      Bottom guard    │
├──────────────────────┤
│      .data           │
├──────────────────────┤
│      .rodata         │
├──────────────────────┤
│      .text           │
├──────────────────────┤ Start of kernel memory.
│                      │
│                      │
│                      │
│                      │
│                      │
│                      │
│                      │
│                      │ Memory mapped I/O devices
│                      │
│                      │
│                      │
│                      │
│                      │
│                      │
│                      │
│                      │
└──────────────────────┘
```
