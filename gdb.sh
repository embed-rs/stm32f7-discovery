#!/bin/bash

set -e

echo "Please run openocd in another terminal window (you might need sudo)"
echo ""

# https://stackoverflow.com/questions/3466166/how-to-check-if-running-in-cygwin-mac-or-linux
unameOut="$(uname -s)"
gdb-multiarch -iex 'add-auto-load-safe-path .' $1
