//! Logging and printing macros

macro_rules! print
{
    ($($args:tt)+) => ({
        use core::fmt::Write;
        use crate::uart;
        // LSP is confused by macros, this unsafe is required
        #[allow(unused_unsafe)]
        let mut dev = unsafe {uart::WRITER.lock()};
        let _ = write!(dev, $($args)+);
        // let _ = write!(uart::Uart::new().lock(), $($args)+);
    });
}

macro_rules! println
{
    () => ({
        print!("\r\n")
    });
    ($fmt:expr) => ({
        print!(concat!($fmt, "\r\n"))
    });
    ($fmt:expr, $($args:tt)+) => ({
        print!(concat!($fmt, "\r\n"), $($args)+)
    });
}

pub enum LogSeverity {
    Debug,
    Info,
    Warning,
    Error,
}

// use as `log::log!(Warning, "This is a test of the warning logging!");`
// in a while that has
// ```
// #[macro_use]
// pub mod log;
// ```
// at the top

macro_rules! log
{
    (Debug, $fmt:expr) => ({
        print!(concat!("[DEBUG] ", $fmt, "\r\n"))
    });
    (Info, $fmt:expr) => ({
        print!(concat!("[INFO] ", $fmt, "\r\n"))
    });
    (Warning, $fmt:expr) => ({
        print!(concat!("[WARN] ", $fmt, "\r\n"))
    });
    (Error, $fmt:expr) => ({
        print!(concat!("[ERROR] ", $fmt, "\r\n"))
    });

    (Debug, $fmt:expr, $($args:tt)+) => ({
        print!(concat!("[DEBUG] ", $fmt, "\r\n"), $($args)+)
    });
    (Info, $fmt:expr, $($args:tt)+) => ({
        print!(concat!("[INFO] ", $fmt, "\r\n"), $($args)+)
    });
    (Warning, $fmt:expr, $($args:tt)+) => ({
        print!(concat!("[WARN] ", $fmt, "\r\n"), $($args)+)
    });
    (Error, $fmt:expr, $($args:tt)+) => ({
        print!(concat!("[ERROR] ", $fmt, "\r\n"), $($args)+)
    });
}

pub(crate) use log;
