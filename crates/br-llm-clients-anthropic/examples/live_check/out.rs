use std::fmt;
use std::io::Write;

pub(crate) fn line(args: fmt::Arguments<'_>) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{args}").and_then(|()| out.flush());
}

pub(crate) fn part(text: &str) {
    let mut out = std::io::stdout().lock();
    let _ = write!(out, "{text}").and_then(|()| out.flush());
}
