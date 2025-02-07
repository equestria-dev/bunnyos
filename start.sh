#!/bin/bash
./build.sh
ssh jbc-host "/Applications/TigerVNC\ Viewer\ 1.14.1.app/Contents/MacOS/TigerVNC\ Viewer" 192.168.1.50:5900&
qemu-system-x86_64 -m 1024M -drive if=pflash,format=raw,readonly=on,file=./firmware/OVMF_CODE.fd -drive if=pflash,format=raw,file=./firmware/OVMF_VARS.fd -device virtio-vga -device qemu-xhci -device usb-tablet -drive file=fat:rw:./esp,format=raw,media=disk -vnc :0,password=off
