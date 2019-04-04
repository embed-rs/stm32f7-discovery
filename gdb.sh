#!/bin/sh

echo "Please run openocd in another terminal window (you might need sudo)"
echo ""

for GDB in arm-none-eabi-gdb gdb-multiarch
do
command -v "$GDB" >/dev/null && break
done

exec "$GDB" -iex 'add-auto-load-safe-path .' "$1"
