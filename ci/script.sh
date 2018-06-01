set -euxo pipefail

main() {
    cargo build
    cargo build --release
}

main
