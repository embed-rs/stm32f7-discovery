set -euxo pipefail

main() {
    cargo build
    cargo build --release
    cargo build --examples
    cargo build --examples --release
}

main
