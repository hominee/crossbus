#![allow(unused)]

#[cfg(all(feature = "time", feature = "wasm32"))]
pub mod time_wasm32;

use core::sync::atomic::{AtomicBool, Ordering};

static LOGGER_INIT: AtomicBool = AtomicBool::new(false);

pub fn init() {
    init_logger();
}

pub fn init_logger() {
    #[cfg(not(feature = "wasm32"))]
    if !LOGGER_INIT.load(Ordering::SeqCst) {
        simple_logger::init_with_level(log::Level::Debug).unwrap();
        log::info!("init logger");
        LOGGER_INIT.store(true, Ordering::SeqCst);
    }

    #[cfg(feature = "wasm32")]
    if !LOGGER_INIT.load(Ordering::SeqCst) {
        wasm_logger::init(wasm_logger::Config::default());
        log::info!("init logger");
        LOGGER_INIT.store(true, Ordering::SeqCst);
    }
}

pub fn init_module_level(module: &str, level: log::Level) {
    #[cfg(not(feature = "wasm32"))]
    if !LOGGER_INIT.load(Ordering::SeqCst) {
        simple_logger::SimpleLogger::new()
            .with_level(log::LevelFilter::Off)
            .with_module_level(module, level.to_level_filter())
            .init()
            .unwrap();
        log::info!("init logger");
        LOGGER_INIT.store(true, Ordering::SeqCst);
    }

    #[cfg(feature = "wasm32")]
    if !LOGGER_INIT.load(Ordering::SeqCst) {
        wasm_logger::init(wasm_logger::Config::new(level).module_prefix(module));
        log::info!("init logger");
        LOGGER_INIT.store(true, Ordering::SeqCst);
    }
}

#[cfg(feature = "wasm32")]
pub fn now() -> f64 {
    use js_sys::Date;

    Date::now() / 1000.0
}

pub fn get_now() -> f64 {
    #[cfg(any(feature = "tokio", feature = "async-std"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    }
    #[cfg(feature = "wasm32")]
    {
        now()
    }
    #[cfg(not(any(feature = "wasm32", feature = "tokio", feature = "async-std")))]
    unimplemented!()
}
