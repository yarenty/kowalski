use env_logger::Builder;
use log::LevelFilter;
use std::io::Write;

/// Initialize the logging system with default settings
pub fn init() {
    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();
}

/// Initialize the logging system with custom settings
pub fn init_with_level(level: LevelFilter) {
    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .filter(None, level)
        .init();
}

/// Initialize the logging system with custom settings and module filters
pub fn init_with_filters(filters: &[(&str, LevelFilter)]) {
    let mut builder = Builder::new();
    builder.format(|buf, record| {
        writeln!(
            buf,
            "{} [{}] - {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.args()
        )
    });

    for (module, level) in filters {
        builder.filter(Some(module), *level);
    }

    builder.init();
} 