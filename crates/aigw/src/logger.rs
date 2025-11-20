use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::fs;
use std::io;
use std::sync::{Arc, Mutex};
use time::format_description::FormatItem;
use tracing::Level;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::NonBlocking;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::MakeWriter;

static WRITER_CACHE: Lazy<DashMap<String, NonBlocking>> = Lazy::new(DashMap::new);
static GUARD_CACHE: Lazy<Mutex<Vec<WorkerGuard>>> = Lazy::new(|| Mutex::new(Vec::new()));

static TIME_FORMAT: &[FormatItem] = time::macros::format_description!(
    "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
);

pub struct MultiFileWriter {
    log_dir: Arc<String>,
}

impl MultiFileWriter {
    fn get_writer(&self, key: &str) -> tracing_appender::non_blocking::NonBlocking {
        if let Some(writer) = WRITER_CACHE.get(key) {
            return writer.clone();
        }

        fs::create_dir_all(&*self.log_dir).ok();

        let file_appender =
            tracing_appender::rolling::daily(&*self.log_dir, format!("{}.log", key));
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        WRITER_CACHE.insert(key.to_string(), non_blocking.clone());
        GUARD_CACHE.lock().unwrap().push(guard);

        non_blocking
    }
}

impl<'a> MakeWriter<'a> for MultiFileWriter {
    type Writer = Box<dyn io::Write + 'a>;

    fn make_writer_for(&'a self, meta: &tracing::Metadata<'_>) -> Self::Writer {
        let key = if *meta.level() == Level::ERROR {
            "error"
        } else {
            match meta.target() {
                "access" => "access",
                "test" => "test",
                "console" => "console",
                _ => "default",
            }
        };

        Box::new(self.get_writer(key))
    }

    fn make_writer(&'a self) -> Self::Writer {
        Box::new(self.get_writer("default"))
    }
}

pub fn init_logger(log_dir: &str) {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    let timer = fmt::time::OffsetTime::new(
        time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC),
        TIME_FORMAT,
    );

    tracing_subscriber::fmt()
        .with_timer(timer)
        .with_writer(MultiFileWriter {
            log_dir: Arc::new(log_dir.to_string()),
        })
        .with_env_filter(filter)
        .init();
}
