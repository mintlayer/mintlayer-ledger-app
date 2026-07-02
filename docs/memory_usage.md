# Memory usage

## Device memory size

Judging by `.ld` files in [the Ledger Rust SDK](https://github.com/LedgerHQ/ledger-device-rust-sdk/tree/cad196841dbd72c037cfa01bec81a4a3ae57a04e/ledger_secure_sdk_sys/devices),
the amount of SRAM each model has is:
| Device            | SRAM |
| ----------------- | ---- |
| apex_p, nanosplus | 40KB |
| flex, stax        | 36KB |
| nanox             | 28KB |

The first part of the RAM will be occupied by the app's globals (one of which will be the heap used by the Rust code) and the rest is stack.

The `HEAP_SIZE` variable in `.cargo/config.toml` specifies the size of the Rust heap (which is just [a static array under the hood](https://github.com/LedgerHQ/ledger-device-rust-sdk/blob/cad196841dbd72c037cfa01bec81a4a3ae57a04e/ledger_secure_sdk_sys/src/lib.rs#L64)).

I.e. the bigger `HEAP_SIZE` is, the less is the stack. And with the `HEAP_SIZE` of 16KB, we'll have less than 12KB of stack at nanox.

## Reducing app stack usage

- Function's parameters and the return value consume stack space (unless the object is small enough to be put into a register).
- Moving an object around inside the function body may increase stack consumption as well.

So,
- Box large types if you need to pass/return them by value.
- Avoid unboxing boxed large objects when passing them by value. E.g. even if a function only needs `LargeObj`,
  pass `Box<LargeObj>` to it anyway (which would be discouraged by the "normal" best practices), because passing it
  unboxed would increase the stack usage.\
  This includes the case when a member function consumes `self` - declare it as `self: Box<Self>` instead.
- `sizeof` of 200 bytes is probably large enough. E.g. in the past boxing certain objects of roughly this size
  decreased stack usage by roughly 1.3KB (which is more than 10% of all stack space available on nanox).

### Determining the current stack usage of the app

Build the app with `emit-stack-sizes`:
```
RUSTFLAGS="-Z emit-stack-sizes" cargo ledger build nanox
```
After that you can use `llvm-readobj` to obtain sizes of stack frames of each function:
```
llvm-readobj --stack-sizes --demangle target/nanox/release/mintlayer-app
```

You can also force `llvm-readobj` to emit json and use `jq` to sort the output by the stack size. E.g. the following
will print 20 functions with the biggest stack frame size:
```
llvm-readobj --stack-sizes --demangle --elf-output-style=JSON target/nanox/release/mintlayer-app | jq -r '.[].StackSizes | sort_by(.Entry.Size) | reverse | .[:20][] | .Entry | "\(.Size)\t\(.Functions | join(", "))"'
```

### Determining the actual available stack

At least in the current version of the SDK, the linker script emits symbols that
can be used to determine the actual stack size, e.g. via `llvm-readelf`:
```
llvm-readelf -s target/nanox/release/mintlayer-app | rg '_stack|_estack'
```
Example output:
```
1581: da7a425c     0 NOTYPE  GLOBAL DEFAULT     6 app_stack_canary
1624: da7a7000     0 NOTYPE  GLOBAL DEFAULT     6 _estack
1697: da7a4260     0 NOTYPE  GLOBAL DEFAULT     6 _stack
```
Here `_estack` is the end of the stack area, `_stack` is the beginning of it and `app_stack_canary` is a 4-byte marker
placed just below `_stack` and used to detect stack overflows. The difference between `_estack` and `_stack` will be
the stack size, in this case it's da7a7000-da7a4260=2DA0 (11680 in decimal).

### Other notes

This code:
```
fn foo(x: &X) {
    match x {
        X::A => { /*do stuff*/ },
        X::B => { /*do other stuff*/ },
    }
}
```
may use more stack than:
```
fn foo(x: &X) {
    match x {
        X::A => stuff(),
        X::B => other_stuff(),
    }
}

#[inline(never)] fn stuff()       { /*do stuff*/ }
#[inline(never)] fn other_stuff() { /*do other stuff*/ }
```
I.e. it seems that LLVM cannot always reuse stack slots between different branches of the `match`, and with bigger enums
and bigger stack usage in each branch the overhead becomes bigger as well. So, splitting a large `match` into separate
non-inlinable functions may be a way of reducing the app's stack usage, but this should probably be the last resort,
because if all large objects are boxed, the stack usage in each branch should be relatively small, which will make
the overhead relatively small as well.
