use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use tracing_subscriber::EnvFilter;
use cfg_if::cfg_if;

use crate::get_create_simsapa_dir;

cfg_if! {
    if #[cfg(target_os = "android")] {
        use std::io::Result as IoResult;
        use android_logger::{Config, FilterBuilder};

        #[derive(Clone)]
        struct AndroidLogWriter;

        impl Write for AndroidLogWriter {
            fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
                let msg = std::str::from_utf8(buf).unwrap_or("invalid UTF-8");

                for line in msg.lines() {
                    let line = line.trim();
                    if line.contains("ERROR") {
                        log::error!("{}", line);
                    } else if line.contains("WARN") {
                        log::warn!("{}", line);
                    } else if line.contains("DEBUG") {
                        log::debug!("{}", line);
                    } else if line.contains("TRACE") {
                        log::trace!("{}", line);
                    } else {
                        log::info!("{}", line);
                    }
                }

                Ok(buf.len())
            }

            fn flush(&mut self) -> IoResult<()> {
                Ok(())
            }
        }

        fn platform_setup() {
            android_logger::init_once(
                Config::default()
                    .with_max_level(log::LevelFilter::Debug)
                    .with_tag("simsapa")
                    .with_filter(FilterBuilder::new().parse("debug").build()),
            );
        }

        fn make_writer() -> impl for<'a> tracing_subscriber::fmt::MakeWriter<'a> + Send + Sync {
            let writer = AndroidLogWriter;
            move || writer.clone()
        }
    } else {
        fn platform_setup() {}

        fn make_writer() -> impl for<'a> tracing_subscriber::fmt::MakeWriter<'a> + Send + Sync {
            std::io::stdout
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LogPrecision {
    Seconds,
    Microseconds,
}

impl LogPrecision {
    pub fn unit_str(&self) -> &'static str {
        match self {
            LogPrecision::Seconds => "s",
            LogPrecision::Microseconds => "µs",
        }
    }
}

pub struct TimeLog {
    start_time: Instant,
    prev_time: Mutex<Instant>,
    precision: LogPrecision,
    log_file: PathBuf,
}

impl TimeLog {
    pub fn new(precision: LogPrecision) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = get_create_simsapa_dir()
            .map_err(|e| format!("Failed to get simsapa_dir: {}", e))?;

        std::fs::create_dir_all(&data_dir)?;
        let log_file = data_dir.join("profile_time.dat");

        let now = Instant::now();

        Ok(TimeLog {
            start_time: now,
            prev_time: Mutex::new(now),
            precision,
            log_file,
        })
    }

    pub fn start(&mut self, start_new: bool) -> Result<(), Box<dyn std::error::Error>> {
        self.start_time = Instant::now();
        if let Ok(mut prev) = self.prev_time.lock() {
            *prev = self.start_time;
        }

        if start_new {
            // Not checking with .exists() b/c of Android permission errors.
            // Try to remove but don't return on error, assume that writing it is still possible.
            match std::fs::remove_file(&self.log_file) {
                Ok(_) => {},
                Err(e) => error(&format!("{}", e)),
            }
        }

        Ok(())
    }

    pub fn log(&self, msg: &str) -> Result<(), Box<dyn std::error::Error>> {
        let trimmed_msg = if msg.len() > 30 {
            &msg[0..30]
        } else {
            msg
        };

        let now = Instant::now();
        let total_elapsed = now.duration_since(self.start_time);

        let step_elapsed = if let Ok(mut prev) = self.prev_time.lock() {
            let elapsed = now.duration_since(*prev);
            *prev = now;
            elapsed
        } else {
            Duration::from_secs(0)
        };

        let (total_time, step_time) = match self.precision {
            LogPrecision::Seconds => {
                (total_elapsed.as_secs(), step_elapsed.as_secs())
            }
            LogPrecision::Microseconds => {
                (total_elapsed.as_micros() as u64, step_elapsed.as_micros() as u64)
            }
        };

        // Escape underscores for compatibility with original format
        let escaped_msg = trimmed_msg.replace('_', "\\_");
        let dat_line = format!("{}\t{}\t{}\n", total_time, step_time, escaped_msg);

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)?;
        file.write_all(dat_line.as_bytes())?;

        Ok(())
    }
}

pub struct Logger {
    log_file: PathBuf,
    time_log: Option<Arc<Mutex<TimeLog>>>,
    disable_log: bool,
    enable_print_log: bool,
}

impl Logger {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = get_create_simsapa_dir()
            .map_err(|e| format!("Failed to get simsapa_dir: {}", e))?;

        std::fs::create_dir_all(&data_dir)?;
        let log_file = data_dir.join("log.txt");

        // Read environment variables
        let disable_log = std::env::var("DISABLE_LOG")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);


        cfg_if! {
            if #[cfg(target_os = "android")] {
                let enable_print_log = true;
            } else {
                let enable_print_log = std::env::var("ENABLE_PRINT_LOG")
                    .map(|v| v.to_lowercase() == "true")
                    .unwrap_or(false);
            }
        }

        let enable_time_log = std::env::var("ENABLE_TIME_LOG")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);

        let time_log = if enable_time_log {
            let mut tl = TimeLog::new(LogPrecision::Microseconds)?;
            tl.start(true)?;
            Some(Arc::new(Mutex::new(tl)))
        } else {
            None
        };

        Ok(Logger {
            log_file,
            time_log,
            disable_log,
            enable_print_log,
        })
    }

    pub fn init_tracing() -> Result<(), Box<dyn std::error::Error>> {
        platform_setup();

        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"));

        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_target(false)
            .with_thread_ids(true)
            .with_file(false)
            .with_line_number(false)
            .with_writer(make_writer())
            .finish();

        tracing::subscriber::set_global_default(subscriber)?;

        Ok(())
    }

    fn write_to_file(&self, message: &str, start_new: bool) -> Result<(), Box<dyn std::error::Error>> {
        if self.disable_log {
            return Ok(());
        }

        let mut file = if start_new {
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&self.log_file)?
        } else {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.log_file)?
        };

        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3fZ");
        let log_line = format!("[{}] {}\n", timestamp, message);

        file.write_all(log_line.as_bytes())?;

        Ok(())
    }

    pub fn info(&self, msg: &str, start_new: bool) {
        let formatted_msg = format!("INFO: {}", msg);

        if self.enable_print_log {
            tracing::info!("{}", msg);
        }

        if let Err(e) = self.write_to_file(&formatted_msg, start_new) {
            eprintln!("Failed to write to log file: {}", e);
        }
    }

    pub fn warn(&self, msg: &str, start_new: bool) {
        let formatted_msg = format!("WARN: {}", msg);

        if self.enable_print_log {
            tracing::warn!("{}", msg);
        }

        if let Err(e) = self.write_to_file(&formatted_msg, start_new) {
            eprintln!("Failed to write to log file: {}", e);
        }
    }

    pub fn error(&self, msg: &str, start_new: bool) {
        let formatted_msg = format!("ERROR: {}", msg);

        if self.enable_print_log {
            tracing::error!("{}", msg);
        }

        if let Err(e) = self.write_to_file(&formatted_msg, start_new) {
            eprintln!("Failed to write to log file: {}", e);
        }
    }

    pub fn profile(&self, msg: &str, start_new: bool) {
        if let Some(time_log) = &self.time_log {
            if let Ok(tl) = time_log.lock() {
                if let Err(e) = tl.log(msg) {
                    eprintln!("Failed to write to profile log: {}", e);
                }
            }
        }

        let elapsed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0));

        let profile_msg = format!("PROFILE: {}: {:?}", msg, elapsed);

        if self.enable_print_log {
            tracing::debug!("{}", profile_msg);
        }

        if let Err(e) = self.write_to_file(&profile_msg, start_new) {
            eprintln!("Failed to write to log file: {}", e);
        }
    }
}

// Global logger instance using OnceLock for thread-safe initialization
static LOGGER: OnceLock<Logger> = OnceLock::new();

pub fn init_logger() -> Result<(), Box<dyn std::error::Error>> {
    if LOGGER.get().is_none() {
        Logger::init_tracing()?;
        let logger = Logger::new()?;
        LOGGER.set(logger).map_err(|_| "Logger already initialized")?;
    }
    Ok(())
}

/// C-compatible logger initialization function
/// Returns 0 on success, 1 on error
#[unsafe(no_mangle)]
pub extern "C" fn init_logger_c() -> i32 {
    match init_logger() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Logger initialization failed: {}", e);
            1
        }
    }
}

/// Check if logger is already initialized
/// Returns 1 if initialized, 0 if not
#[unsafe(no_mangle)]
pub extern "C" fn is_logger_initialized() -> i32 {
    if LOGGER.get().is_some() { 1 } else { 0 }
}

fn with_logger<F, R>(f: F) -> R
where
    F: FnOnce(&Logger) -> R,
    R: Default,
{
    match LOGGER.get() {
        Some(logger) => f(logger),
        None => {
            eprintln!("Logger not initialized! Call init_logger() first.");
            R::default()
        }
    }
}

// Public API functions
pub fn info(msg: &str) {
    info_with_options(msg, false);
}

pub fn info_with_options(msg: &str, start_new: bool) {
    with_logger(|logger| logger.info(msg, start_new));
}

pub fn warn(msg: &str) {
    warn_with_options(msg, false);
}

pub fn warn_with_options(msg: &str, start_new: bool) {
    with_logger(|logger| logger.warn(msg, start_new));
}

pub fn error(msg: &str) {
    error_with_options(msg, false);
}

pub fn error_with_options(msg: &str, start_new: bool) {
    with_logger(|logger| logger.error(msg, start_new));
}

pub fn profile(msg: &str) {
    profile_with_options(msg, false);
}

pub fn profile_with_options(msg: &str, start_new: bool) {
    with_logger(|logger| logger.profile(msg, start_new));
}

// Utility function to format duration similar to the Python strfdelta
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

// C-compatible logging functions
use std::ffi::CStr;
use std::os::raw::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn log_info_c(msg: *const c_char) {
    if msg.is_null() {
        return;
    }
    let c_str = unsafe { CStr::from_ptr(msg) };
    if let Ok(rust_str) = c_str.to_str() {
        info(rust_str);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_info_with_options_c(msg: *const c_char, start_new: bool) {
    if msg.is_null() {
        return;
    }
    let c_str = unsafe { CStr::from_ptr(msg) };
    if let Ok(rust_str) = c_str.to_str() {
        info_with_options(rust_str, start_new);
    }
}
