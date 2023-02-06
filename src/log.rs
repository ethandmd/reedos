/*
 * module for logging generally
 */

macro_rules! print
{
    ($($args:tt)+) => ({
        use core::fmt::Write;
        let _ = write!(uart::Uart::new(0x1000_0000), $($args)+);
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

pub enum Log_Severity {
    Debug,
    Info,
    Warning,
    Error
}

macro_rules! log
{
    (sev: Log_Severity, $fmt:expr) => ({
	let _sev_str = match sev {
	    Debug => "[DEBUG] ",
	    Info => "[INFO]",
	    Warning => "[WARN]",
	    Error => "[ERROR]"
	};
	
	print!(concat!(_sev_str, $fmt, "\r\n"))
    });
    (sev: Log_Severity, $fmt:expr, $($args:tt)+) => ({
	let _sev_str = match sev {
	    Debug => "[DEBUG] ",
	    Info => "[INFO]",
	    Warning => "[WARN]",
	    Error => "[ERROR]"
	};

	
	print!(concat!(_sev_str, $fmt, "\r\n"), $($args)+)
    })
}


