# Setup

1. install stlink
    * arch: `sudo pacman -S stlink`
    * general linux
        * install `libusb-dev` 1.0
        * install `cmake`
        * install a C-Compiler (sorry)
        * `git clone https://github.com/texane/stlink.git && cd stlink && make release && cd build/Release && sudo make install`
    * mac os: `brew install stlink`
-  install arm cross compilers
    * debian/ubuntu: `sudo apt-get install gcc-arm-none-eabi gdb-arm-none-eabi`
    * macOS: `brew tap osx-cross/arm && brew install arm-gcc-bin`
-  install the ARM rust toolchain
    * `rustup toolchain install nightly-arm-unknown-linux-gnueabi`
-  install a nightly compiler
    * `rustup update nightly`
-  dowload the rust source code
    * if your rustup does not have the `component` subcommand: `rustup self update`
    * `rustup component add rust-src --toolchain nightly`
-  install `xargo`
    * `rustup run nightly cargo install xargo`
    * NOTE: do **not** run this command in the `novemb-rs-stm32f7` folder, you will get errors about the compiler not finding the standard library
-  get the demo code
    * `git clone https://github.com/embed-rs/novemb-rs-stm32f7.git`

# Compiling

1. `cd novemb-rs-stm32f7`
-  `rustup override set nightly`
-  `xargo build`
-  have patience, the first time you run `xargo build`, the `core` library and various others need to be built.
-  open another terminal and run `st-util`
-  go back to your first terminal
-  run `sh gdb.sh`
-  The code has now been flashed and is ready to run. Type `c` (for `continue`) and observe your controller.

# Generate Documentation

`cargo doc --open --target x86_64-unknown-linux-gnu`
