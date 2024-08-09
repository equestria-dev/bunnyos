cd ./os
cargo build --target x86_64-unknown-uefi
cd ../tools
cargo build
cd ..
./tools/target/debug/mkbimg.exe
