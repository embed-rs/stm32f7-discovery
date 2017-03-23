# Setup

1. install stlink
    * arch: `sudo pacman -S stlink`
    * general linux
        * install `libusb-dev` 1.0
        * install `cmake`
        * install a C-Compiler (sorry)
        * `git clone https://github.com/texane/stlink.git && cd stlink && make release && cd build/Release && sudo make install`
    * mac os: `brew install stlink`
    * windows: unzip `stlink-1.3.1-win32.zip`
-  install arm cross compilers
    * debian/ubuntu: `sudo apt-get install gcc-arm-none-eabi gdb-arm-none-eabi`
    * macOS: `brew tap osx-cross/arm && brew install arm-gcc-bin`
    * windows:
        * download `GNU ARM Embedded Toolchain` from https://developer.arm.com/open-source/gnu-toolchain/gnu-rm/downloads
        * execute to install
        * ensure installation path is added to 'PATH' variable (might require a reboot)
-  install the ARM rust toolchain
    * `rustup toolchain install nightly-arm-unknown-linux-gnueabi`
-  install a nightly compiler
    * `rustup update nightly`
-  dowload the rust source code
    * if your rustup does not have the `component` subcommand: `rustup self update`
    * `rustup component add rust-src --toolchain nightly`
-  install `xargo`
    * `rustup run nightly cargo install xargo`
    * NOTE: do **not** run this command in the `stm32f7_discovery` folder, you will get errors about the compiler not finding the standard library
-  get the demo code
    * `git clone https://github.com/embed-rs/stm32f7-discovery.git`

# Compiling

1. `cd stm32f7_discovery`
-  `rustup override set nightly`
-  `xargo build`
-  have patience, the first time you run `xargo build`, the `core` library and various others need to be built.
-  open another terminal and run `st-util` (win: `st-util.exe` is located in `stlink-1.3.1-win32\bin`, which was unziped for setup)
-  go back to your first terminal
-  run `sh gdb.sh` (run `gdb.bat` for win)
-  The code has now been flashed and is ready to run. Type `c` (for `continue`) and observe your controller.

# Generate Documentation

`cargo doc --open --target x86_64-unknown-linux-gnu`
