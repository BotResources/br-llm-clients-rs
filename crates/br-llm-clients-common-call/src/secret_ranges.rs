use std::ops::Range;

use crate::secret_automaton::{Automaton, ROOT};

#[derive(Default)]
struct Merged {
    ranges: Vec<Range<usize>>,
    covered: usize,
}

impl Merged {
    fn add(&mut self, start: usize, end: usize) {
        let mut start = start;
        while let Some(last) = self.ranges.last().filter(|last| last.end > start) {
            start = start.min(last.start);
            self.covered = self.covered.saturating_sub(last.len());
            self.ranges.pop();
        }
        self.covered = self.covered.saturating_add(end.saturating_sub(start));
        self.ranges.push(start..end);
    }
}

struct Scan {
    longest_by_end: Vec<usize>,
    all: Merged,
    open_from: usize,
}

fn scan(automaton: &Automaton, bytes: &[u8]) -> Scan {
    let mut longest_by_end = Vec::with_capacity(bytes.len());
    let mut all = Merged::default();
    let mut state = ROOT;
    for (end, byte) in (1_usize..).zip(bytes) {
        state = automaton.step(state, *byte);
        let longest = automaton.longest_ending(state);
        if longest > 0 {
            all.add(end.saturating_sub(longest), end);
        }
        longest_by_end.push(longest);
    }
    Scan {
        longest_by_end,
        all,
        open_from: bytes.len().saturating_sub(automaton.open_depth(state)),
    }
}

pub(crate) fn secret_ranges(automaton: Option<&Automaton>, bytes: &[u8]) -> Vec<Range<usize>> {
    match automaton {
        Some(automaton) => scan(automaton, bytes).all.ranges,
        None if bytes.is_empty() => Vec::new(),
        None => std::iter::once(0..bytes.len()).collect(),
    }
}

pub(crate) struct Cuts {
    pub(crate) safe: Vec<bool>,
    pub(crate) covered: Vec<bool>,
}

pub(crate) fn cuts(automaton: &Automaton, bytes: &[u8]) -> Cuts {
    let scan = scan(automaton, bytes);
    let mut covered = vec![false; bytes.len()];
    for range in scan
        .all
        .ranges
        .iter()
        .cloned()
        .chain(std::iter::once(scan.open_from..bytes.len()))
    {
        covered.get_mut(range).unwrap_or_default().fill(true);
    }
    let mut safe = vec![false; bytes.len().saturating_add(1)];
    if let Some(first) = safe.first_mut() {
        *first = true;
    }
    let mut whole = scan.all.ranges.iter().peekable();
    let mut covered_below = 0_usize;
    let mut ended_below = Merged::default();
    for (end, longest) in (1_usize..=scan.open_from).zip(&scan.longest_by_end) {
        let byte = end.saturating_sub(1);
        while whole.next_if(|range| range.end <= byte).is_some() {}
        if whole.peek().is_some_and(|range| range.contains(&byte)) {
            covered_below = covered_below.saturating_add(1);
        }
        if *longest > 0 {
            ended_below.add(end.saturating_sub(*longest), end);
        }
        if let Some(flag) = safe.get_mut(end) {
            *flag = ended_below.covered == covered_below;
        }
    }
    Cuts { safe, covered }
}

pub(crate) fn kept_before_cut(automaton: Option<&Automaton>, bytes: &[u8]) -> usize {
    automaton.map_or(0, |automaton| last_safe(&cuts(automaton, bytes).safe))
}

pub(crate) fn last_safe(safe: &[bool]) -> usize {
    safe.iter().rposition(|safe| *safe).unwrap_or(0)
}
