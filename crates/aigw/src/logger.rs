use std::collections::HashMap;
use std::fs;
use std::io;
use std::sync::{Arc, Mutex};
use time::format_description::FormatItem;
use tracing::Level;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::MakeWriter;

static WRITER_CACHE: once_cell::sync::Lazy<
    Arc<Mutex<HashMap<String, tracing_appender::non_blocking::NonBlocking>>>,
> = once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

static GUARD_CACHE: once_cell::sync::Lazy<
    Arc<Mutex<Vec<tracing_appender::non_blocking::WorkerGuard>>>,
> = once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

static TIME_FORMAT: &[FormatItem] = time::macros::format_description!(
    "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
);

pub struct MultiFileWriter {
    log_dir: String,
}

impl<'a> MakeWriter<'a> for MultiFileWriter {
    type Writer = Box<dyn io::Write + 'a>;

    fn make_writer_for(&'a self, meta: &tracing::Metadata<'_>) -> Self::Writer {
        let key = if *meta.level() == Level::ERROR {
            "error".to_string()
        } else if meta.target() == "access" {
            "access".to_string()
        } else {
            "default".to_string()
        };

        let mut cache = WRITER_CACHE.lock().unwrap();
        if !cache.contains_key(&key) {
            fs::create_dir_all(&self.log_dir).ok();
            let file_appender =
                tracing_appender::rolling::daily(&self.log_dir, format!("{}.log", key));
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

            cache.insert(key.clone(), non_blocking);
            GUARD_CACHE.lock().unwrap().push(guard);
        }

        Box::new(cache.get(&key).unwrap().clone())
    }

    fn make_writer(&'a self) -> Self::Writer {
        // fallback to default
        let key = "default";

        let mut cache = WRITER_CACHE.lock().unwrap();
        if !cache.contains_key(key) {
            fs::create_dir_all(&self.log_dir).ok();
            let file_appender =
                tracing_appender::rolling::daily(&self.log_dir, format!("{}.log", key));
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            cache.insert(key.to_string(), non_blocking.clone());
            GUARD_CACHE.lock().unwrap().push(guard);
        }
        Box::new(cache.get(key).unwrap().clone())
    }
}

pub fn init_logger(log_dir: &str) {
    let filter = EnvFilter::from_default_env()
        .add_directive("access=info".parse().unwrap())
        .add_directive(LevelFilter::INFO.into());

    let timer = fmt::time::OffsetTime::new(
        time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC),
        TIME_FORMAT,
    );

    tracing_subscriber::fmt()
        .with_timer(timer)
        .with_writer(MultiFileWriter {
            log_dir: log_dir.to_string(),
        })
        .with_env_filter(filter)
        .init();
}
