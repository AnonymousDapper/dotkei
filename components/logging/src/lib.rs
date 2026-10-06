// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

use std::collections::BTreeMap;
use std::fmt::Write as _;

use colored::{Color, Colorize};

pub use log::{Level, LevelFilter, debug, error, info, trace, warn};

use log::Record;

use log::kv::{self, Key, Value, VisitSource};

struct KVCollate<'kv>(BTreeMap<Key<'kv>, Value<'kv>>);

impl<'kv> VisitSource<'kv> for KVCollate<'kv> {
    fn visit_pair(&mut self, key: Key<'kv>, value: Value<'kv>) -> Result<(), kv::Error> {
        self.0.insert(key, value);

        Ok(())
    }
}

struct Style {
    fg: Option<Color>,
    bg: Option<Color>,
    style: colored::Style,
}

impl Style {
    fn with_fg(fg: Color) -> Style {
        Style {
            fg: Some(fg),
            bg: None,
            style: colored::Style::default(),
        }
    }

    fn with_bg(bg: Color) -> Style {
        Style {
            fg: None,
            bg: Some(bg),
            style: colored::Style::default(),
        }
    }

    fn with_colors(fg: Color, bg: Color) -> Style {
        Style {
            fg: Some(fg),
            bg: Some(bg),
            style: colored::Style::default(),
        }
    }

    fn with_style(style: colored::Style) -> Style {
        Style {
            fg: None,
            bg: None,
            style,
        }
    }

    fn set_fg(mut self, fg: Color) -> Self {
        self.fg = Some(fg);
        self
    }

    fn set_bg(mut self, bg: Color) -> Self {
        self.bg = Some(bg);
        self
    }

    fn set_style(mut self, style: colored::Style) -> Self {
        self.style = style;
        self
    }

    fn add_style(mut self, style: colored::Styles) -> Self {
        self.style |= style;
        self
    }

    fn apply<S: Into<String>>(&self, input: S) -> colored::ColoredString {
        let mut output = colored::ColoredString::from(input.into());

        output.fgcolor = self.fg;
        output.bgcolor = self.bg;
        output.style = self.style;

        output
    }
}

fn level_style(level: &Level) -> Style {
    match level {
        Level::Error => Style::with_colors(Color::BrightWhite, Color::Red)
            .set_style(colored::Styles::Bold | colored::Styles::Underline),
        Level::Warn => {
            Style::with_colors(Color::BrightWhite, Color::Yellow).add_style(colored::Styles::Bold)
        }
        Level::Info => {
            Style::with_colors(Color::Green, Color::BrightWhite).add_style(colored::Styles::Bold)
        }
        Level::Debug => Style::with_fg(Color::BrightBlue),
        Level::Trace => Style::with_fg(Color::BrightMagenta),
    }
}

fn level_token(level: &Level) -> &'static str {
    match level {
        Level::Error => "E",
        Level::Warn => "W",
        Level::Info => "?",
        Level::Debug => "D",
        Level::Trace => "T",
    }
}

fn prefix_token(level: &Level) -> String {
    format!("[{}]", level_token(level))
}

fn target_token(target: &str) -> String {
    target.italic().to_string()
}

fn separator_token(style: &Style) -> String {
    format!("\n  {} ", style.apply("|"))
}

fn location_token(record: &Record) -> String {
    let mut buffer = String::from("{ ");

    // let _ = write!(
    //     &mut buffer,
    //     "{} ",
    //     record.module_path().unwrap_or("<unknown>").underline()
    // );

    if let Some(file) = record.file() {
        let _ = write!(&mut buffer, "{}", file);
    }

    if let Some(line) = record.line() {
        let _ = write!(&mut buffer, ": {} ", line.to_string().underline());
    }

    let _ = write!(&mut buffer, "}}");

    buffer.bright_black().on_bright_white().to_string()
}

fn format_kv(mut buffer: &mut String, record: &Record, sep: String) {
    let mut visitor = KVCollate(BTreeMap::new());

    if record.key_values().visit(&mut visitor).is_ok() {
        for (k, v) in visitor.0.iter() {
            let _ = write!(&mut buffer, "{}{} => {}", sep, k, v);
        }
    }
}

fn format_record(record: &Record) -> String {
    let mut buffer = String::new();

    let style = level_style(&record.level());
    let prefix = style.apply(prefix_token(&record.level()));
    let sep = separator_token(&style);

    let start = style.apply("::");

    let _ = write!(
        &mut buffer,
        "{} {} {} {} {}",
        prefix,
        location_token(record),
        target_token(record.target()),
        &start,
        record.args().to_string().replace('\n', &sep),
    );

    format_kv(&mut buffer, record, sep);

    buffer
}

struct ConsoleLogger;

impl log::Log for ConsoleLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::max_level()
    }
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{}", format_record(record))
        }
    }
    fn flush(&self) {}
}

static LOGGER: ConsoleLogger = ConsoleLogger;

pub fn init(max_level: LevelFilter, use_no_color: bool) -> Result<(), log::SetLoggerError> {
    if use_no_color {
        colored::control::set_override(false);
    }

    log::set_logger(&LOGGER).map(|()| log::set_max_level(max_level))
}
