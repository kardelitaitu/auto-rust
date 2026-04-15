use chrono::Local;
use log::LevelFilter;
use log::{Log, Metadata, Record};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

pub struct FileLogger {
    file: Mutex<File>,
    level: LevelFilter,
}

impl FileLogger {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        Ok(Self {
            file: Mutex::new(file),
            level: LevelFilter::Info,
        })
    }
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let timestamp = Local::now().format("%H:%M:%S").to_string();
            let msg = format!("{} {} {}\n", timestamp, record.level(), record.args());

            // Write to stdout
            print!("{}", msg);

            // Write to file
            if let Ok(mut file) = self.file.lock() {
                let _ = file.write_all(msg.as_bytes());
            }
        }
    }

    fn flush(&self) {
        if let Ok(mut file) = self.file.lock() {
            let _ = file.flush();
        }
    }
}
