rm -rf ./esp
cd ./os || exit 1
cargo build --target x86_64-unknown-uefi || exit 1
cd ../tools || exit 1
cargo build || exit 1
cd .. || exit 1
./tools/target/debug/mkrimg || exit 1
#qemu-system-x86_64 -drive if=pflash,format=raw,readonly=on,file=./firmware/OVMF_CODE.fd -drive if=pflash,format=raw,readonly=on,file=./firmware/OVMF_VARS.fd -drive format=raw,file=fat:rw:./esp
