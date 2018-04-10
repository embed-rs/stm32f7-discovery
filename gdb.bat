echo "Please run 'st-util' in another terminal window"
echo ""
set path=%1
arm-none-eabi-gdb -iex "add-auto-load-safe-path ." -ex "tar ext :4242" -ex "load-reset" %path%
