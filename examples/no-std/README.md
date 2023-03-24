# Intro 

A simple demo that run crossbus on bare metal.

the demo is based on the [cortex-m template](https://github.com/rust-embedded/cortex-m-quickstart), it is recommended to go to the [Rust embedded book](https://docs.rust-embedded.org/book/start/qemu.html) to see around before run it.

# Usage 

before you run it 
some toolchains must be installed:
- [qemu](https://wiki.qemu.org/Documentation/Platforms/ARM#Supported_in_qemu-system-arm), as it is hosted for the compiled binary system
- thumbv7m-none-eabi, compilation target should be added beforehand 
  `rustup target add thumbv7m-none-eabi`
- (Optional) Debugging toolchains, `eg. GDB, OpenOCD`

if any problem happens, the embedded book is recommended to refer to.

With all of obove steps finished, just use 
`cargo r` 
to run it
