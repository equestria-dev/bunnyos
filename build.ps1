cd ./os
if ($?) { cargo build --target x86_64-unknown-uefi } else { cd ..; exit 1 }
if ($?) { cd ../tools } else { cd ..; exit 1 }
if ($?) { cargo build } else { cd ..; exit 1 }
if ($?) { cd .. } else { exit 1 }
if ($?) { ./tools/target/debug/mkbimg.exe } else { exit 1 }
