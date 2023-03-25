//! disable logging out when feature not enabled
//!
#![allow(unused)]
macro_rules! error {
    ($fmt:expr) => {
        ()
    };
    ($fmt:expr, $($args:tt)*) => {
        core::format_args!($fmt, $($args)*);
        ()
    };
}

pub(crate) use error;

macro_rules! warn_log {
    ($fmt:expr) => {
        ()
    };
    ($fmt:expr, $($args:tt)*) => {
        core::format_args!($fmt, $($args)*);
        ()
    };
}

pub(crate) use warn_log as warn;

macro_rules! info {
    ($fmt:expr) => {
        ()
    };
    ($fmt:expr, $($args:tt)*) => {
        core::format_args!($fmt, $($args)*);
        ()
    };
}

pub(crate) use info;

macro_rules! debug {
    ($fmt:expr) => {
        ()
    };
    ($fmt:expr, $($args:tt)*) => {
        core::format_args!($fmt, $($args)*);
        ()
    };
}

pub(crate) use debug;

macro_rules! trace {
    ($fmt:expr) => {
        ()
    };
    ($fmt:expr, $($args:tt)*) => {
        core::format_args!($fmt, $($args)*);
        ()
    };
}

pub(crate) use trace;
