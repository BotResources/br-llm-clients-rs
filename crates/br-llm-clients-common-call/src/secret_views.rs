use std::ops::Range;

use crate::json_view::{DECODINGS, View};
use crate::secret_automaton::Automaton;
use crate::secret_ranges::{cuts, kept_before_cut, last_safe, secret_ranges};

fn views(bytes: &[u8], cut: bool) -> Vec<View> {
    let mut views: Vec<View> = Vec::new();
    if !bytes.contains(&b'\\') {
        return views;
    }
    let raw = View::raw(bytes);
    for _ in 0..DECODINGS {
        let last = views.last().unwrap_or(&raw);
        let limit = if cut { last.whole } else { last.bytes.len() };
        let Some(next) = last.decoded(limit) else {
            break;
        };
        views.push(next);
    }
    views
}

fn merged(mut ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    ranges.sort_unstable_by_key(|range| (range.start, range.end));
    let mut merged: Vec<Range<usize>> = Vec::with_capacity(ranges.len());
    for range in ranges {
        match merged.last_mut() {
            Some(last) if range.start < last.end => last.end = last.end.max(range.end),
            _ => merged.push(range),
        }
    }
    merged
}

pub(crate) fn ranges_in_views(automaton: Option<&Automaton>, bytes: &[u8]) -> Vec<Range<usize>> {
    let found = secret_ranges(automaton, bytes);
    let Some(automaton) = automaton else {
        return found;
    };
    let decoded = views(bytes, false);
    if decoded.is_empty() {
        return found;
    }
    let mut all = found;
    for view in &decoded {
        all.extend(
            secret_ranges(Some(automaton), &view.bytes)
                .into_iter()
                .map(|range| view.source(range)),
        );
    }
    merged(all)
}

fn allowed_by(automaton: &Automaton, view: &View, length: usize) -> Vec<bool> {
    let cuts = cuts(automaton, view.bytes.get(..view.whole).unwrap_or_default());
    let mut allowed = vec![false; length];
    for (at, covered) in cuts.covered.iter().enumerate() {
        if let Some(span) = view.span(at) {
            let inside = span.start.saturating_add(1)..span.end;
            allowed.get_mut(inside).unwrap_or_default().fill(!covered);
        }
    }
    for (at, safe) in cuts.safe.iter().enumerate() {
        let opens = at == 0 || view.span(at) != view.span(at.saturating_sub(1));
        if let Some(flag) = allowed.get_mut(view.source_at(at)).filter(|_| opens) {
            *flag = *safe;
        }
    }
    allowed
}

pub(crate) fn kept_in_views(automaton: Option<&Automaton>, bytes: &[u8]) -> usize {
    let Some(automaton) = automaton else {
        return 0;
    };
    let decoded = views(bytes, true);
    if decoded.is_empty() {
        return kept_before_cut(Some(automaton), bytes);
    }
    let mut kept = cuts(automaton, bytes).safe;
    for view in &decoded {
        let allowed = allowed_by(automaton, view, kept.len());
        for (flag, allowed) in kept.iter_mut().zip(allowed) {
            *flag &= allowed;
        }
    }
    last_safe(&kept)
}
