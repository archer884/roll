use std::{fs::OpenOptions, io, path::Path};

use chrono::Utc;
use hashbrown::HashMap;
use io::BufWriter;

#[derive(Clone, Debug)]
pub struct History<'a> {
    path: &'a Path,
    history: HashMap<i32, Vec<i32>>,
}

impl<'a> History<'a> {
    pub fn new(path: &'a Path) -> Self {
        Self {
            path,
            history: HashMap::new(),
        }
    }

    pub fn append_log(&mut self, log: HashMap<i32, impl AsRef<[i32]>>) {
        for (key, values) in log {
            self.history.entry(key).or_default().extend(values.as_ref());
        }
    }

    pub fn write(self) -> io::Result<()> {
        use std::io::Write;

        static PROGRAM_VERSION: &str = env!("CARGO_PKG_VERSION");

        let mut buf = String::new();
        let mut history = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.path)
            .map(BufWriter::new)?;

        let timestamp = format!("{}", Utc::now().format("%F %R"));

        for (key, values) in self.history {
            writeln!(
                history,
                "{}|{}|{}:{}",
                timestamp,
                PROGRAM_VERSION,
                key,
                format_values(&mut buf, &values)
            )?;
        }

        Ok(())
    }
}

fn format_values<'a>(buf: &'a mut String, values: &[i32]) -> &'a str {
    use std::fmt::Write;

    buf.clear();

    let mut values = values.iter().copied();

    if let Some(first) = values.next() {
        write!(buf, "{}", first).unwrap();
    }

    for n in values {
        write!(buf, ",{}", n).unwrap();
    }

    buf
}
