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
2. install arm cross compilers
    * arch: `sudo pacman -S arm-none-eabi-gcc arm-none-eabi-gdb`
    * debian/ubuntu: `sudo apt-get install gcc-arm-none-eabi gdb-arm-none-eabi`
    * macOS: `brew tap osx-cross/arm && brew install arm-gcc-bin`
    * windows:
        * download `GNU ARM Embedded Toolchain` from https://developer.arm.com/open-source/gnu-toolchain/gnu-rm/downloads
        * execute to install
        * ensure installation path is added to 'PATH' variable (might require a reboot)
3. install a nightly compiler
    * `rustup update nightly`
4. dowload the rust source code
    * if your rustup does not have the `component` subcommand: `rustup self update`
    * `rustup component add rust-src --toolchain nightly`
5. install `xargo`
    * `rustup run nightly cargo install xargo`
    * NOTE: do **not** run this command in the `stm32f7_discovery` folder, you will get errors about the compiler not finding the standard library
6. get the demo code
    * `git clone https://github.com/embed-rs/stm32f7-discovery.git`

# Compiling

1. `cd stm32f7_discovery`
2. `rustup override set nightly`
3. `xargo build`
4. have patience, the first time you run `xargo build`, the `core` library and various others need to be built.
5. open another terminal and run `st-util` (win: `st-util.exe` is located in `stlink-1.3.1-win32\bin`, which was unziped for setup)
6. go back to your first terminal
7. run `sh gdb.sh` (run `gdb.bat` for win)
8. (optional) run `semihosting-enable` to enable [semihosting](http://embed.rs/articles/2016/semi-hosting-rust/) support
9. The code has now been flashed and is ready to run. Type `c` (for `continue`) and observe your controller.

# Generate Documentation

`cargo doc --open --target x86_64-unknown-linux-gnu`
