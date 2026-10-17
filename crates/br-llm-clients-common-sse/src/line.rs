pub(crate) const COLON: u8 = b':';
const SPACE: u8 = b' ';

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Line<'a> {
    Empty,
    Comment,
    Field { name: &'a [u8], value: &'a [u8] },
}

pub(crate) fn classify(line: &[u8]) -> Line<'_> {
    match line.first() {
        None => Line::Empty,
        Some(&COLON) => Line::Comment,
        Some(_) => match line.iter().position(|byte| *byte == COLON) {
            None => Line::Field {
                name: line,
                value: &[],
            },
            Some(colon) => {
                let (name, from_colon) = line.split_at(colon);
                let value = from_colon.get(1..).unwrap_or_default();
                let value = value.strip_prefix(&[SPACE]).unwrap_or(value);
                Line::Field { name, value }
            }
        },
    }
}
