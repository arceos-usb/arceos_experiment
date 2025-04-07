make A=apps/cli PLATFORM=aarch64-qemu-virt LOG=trace XHCI=y QEMU_LOG=y QEMU_CONSOLE=y
qemu-system-aarch64 -m 2G -smp 1 -cpu cortex-a72 -machine virt,highmem=off -kernel apps/cli/cli_aarch64-qemu-virt.bin -device nec-usb-xhci,id=xhci -device usb-mouse,bus=xhci.0 -nographic \
  -monitor unix:/tmp/qemu-monitor-socket,server,nowait \
  -D qemu.log
