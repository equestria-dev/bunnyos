#[macro_export]
macro_rules! eprintln {
    ($($arg:tt)*) => ($crate::println!($($arg)*));
}

#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => ($crate::print!($($arg)*));
}

#[macro_export]
macro_rules! dbg {
    () => {
        $crate::eprintln!("[{}:{}:{}]", file!(), line!(), column!())
    };
    ($val:expr $(,)?) => {
        match $val {
            tmp => {
                $crate::eprintln!("[{}:{}:{}] {} = {:#?}",
                    file!(), line!(), column!(), stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg!($val)),+,)
    };
}
