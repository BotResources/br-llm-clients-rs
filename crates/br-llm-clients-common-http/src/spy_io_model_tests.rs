use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use tokio::io::{AsyncRead, ReadBuf};

use crate::h2state::H2State;
use crate::h2state_tests::bare_state;
use crate::spy_io::SettingsSpy;
use crate::support::frames::{ACK, PING, SETTINGS, frame, settings_frame};

const PERMIT_CAP: usize = 12;

enum Step {
    Bytes(Vec<u8>),
    Pending,
}

struct Scripted(VecDeque<Step>);

impl AsyncRead for Scripted {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.0.pop_front() {
            Some(Step::Bytes(bytes)) => {
                buf.put_slice(&bytes);
                Poll::Ready(Ok(()))
            }
            Some(Step::Pending) => Poll::Pending,
            None => Poll::Ready(Ok(())),
        }
    }
}

fn poll_once(spy: &mut SettingsSpy<Scripted>) {
    let mut storage = [0u8; 128];
    let mut buf = ReadBuf::new(&mut storage);
    let mut cx = Context::from_waker(Waker::noop());
    let _ = Pin::new(spy).poll_read(&mut cx, &mut buf);
}

fn obtainable_permits(state: &Arc<H2State>) -> usize {
    let permits: Vec<_> = (0..PERMIT_CAP).map_while(|_| state.try_acquire()).collect();
    permits.len()
}

struct Dice(u64);

impl Dice {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).unwrap_or(1)).unwrap_or(0)
    }
}

#[derive(Clone, Copy)]
struct Completed {
    end: usize,
    limit: Option<Option<u32>>,
}

fn entry(id: u16, value: u32) -> Vec<u8> {
    let mut bytes = id.to_be_bytes().to_vec();
    bytes.extend_from_slice(&value.to_be_bytes());
    bytes
}

fn random_frame(dice: &mut Dice) -> (Vec<u8>, Option<Option<u32>>) {
    match dice.below(5) {
        0 => (frame(SETTINGS, ACK, 0, &[]), None),
        1 => (frame(PING, 0, 0, &[7; 8]), None),
        2 => (frame(SETTINGS, 0, 0, &[1; 7]), None),
        _ => {
            let mut payload = Vec::new();
            let mut limit = None;
            for _ in 0..dice.below(4) {
                let value = u32::try_from(dice.below(20)).unwrap_or(0);
                if dice.below(2) == 0 {
                    payload.extend_from_slice(&entry(3, value));
                    limit = Some(value);
                } else {
                    payload.extend_from_slice(&entry(4, value));
                }
            }
            (frame(SETTINGS, 0, 0, &payload), Some(limit))
        }
    }
}

struct Model {
    known: bool,
    limit: usize,
}

impl Model {
    fn apply(&mut self, limit: Option<u32>) {
        match limit {
            Some(value) => {
                self.limit = usize::try_from(value).unwrap_or(usize::MAX);
                self.known = true;
            }
            None if !self.known => {
                self.limit = usize::MAX;
                self.known = true;
            }
            None => {}
        }
    }
}

#[test]
fn given_random_frame_streams_cut_at_random_places_then_the_limit_follows_the_frames_completed_before_each_poll()
 {
    for seed in 1..=400u64 {
        let mut dice = Dice(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let mut wire = Vec::new();
        let mut completed = Vec::new();
        for _ in 0..=dice.below(6) {
            let (bytes, limit) = random_frame(&mut dice);
            wire.extend_from_slice(&bytes);
            completed.push(Completed {
                end: wire.len(),
                limit,
            });
        }
        let mut steps = VecDeque::new();
        let mut cuts = Vec::new();
        let mut at = 0;
        while at < wire.len() {
            if dice.below(4) == 0 {
                steps.push_back(Step::Pending);
                cuts.push(at);
            }
            let size = (1 + dice.below(40)).min(wire.len() - at);
            steps.push_back(Step::Bytes(wire[at..at + size].to_vec()));
            at += size;
            cuts.push(at);
        }
        steps.push_back(Step::Pending);
        cuts.push(wire.len());
        let state = bare_state();
        let mut spy = SettingsSpy::new(Scripted(steps), Arc::clone(&state));
        let mut model = Model {
            known: false,
            limit: 0,
        };
        let mut applied = 0;
        for poll in 0..cuts.len() {
            let before = completed
                .iter()
                .filter(|frame| frame.end <= fed_before(&cuts, poll))
                .count();
            for frame in &completed[applied..before] {
                if let Some(limit) = frame.limit {
                    model.apply(limit);
                }
            }
            applied = before;
            poll_once(&mut spy);
            assert_eq!(
                state.awaiting_settings(),
                !model.known,
                "seed {seed}, poll {poll}"
            );
            let expected = if model.known {
                model.limit.min(PERMIT_CAP)
            } else {
                0
            };
            assert_eq!(
                obtainable_permits(&state),
                expected,
                "seed {seed}, poll {poll}"
            );
        }
    }
}

fn fed_before(cuts: &[usize], poll: usize) -> usize {
    poll.checked_sub(1)
        .and_then(|previous| cuts.get(previous))
        .copied()
        .unwrap_or(0)
}

fn two_frames(first: Option<u32>, second: Option<u32>) -> Vec<u8> {
    let mut wire = settings_frame(first);
    wire.extend_from_slice(&settings_frame(second));
    wire
}

fn limit_after_one_read_and_one_more(wire: Vec<u8>) -> (bool, usize) {
    let state = bare_state();
    let steps = VecDeque::from([Step::Bytes(wire), Step::Pending]);
    let mut spy = SettingsSpy::new(Scripted(steps), Arc::clone(&state));
    poll_once(&mut spy);
    poll_once(&mut spy);
    (state.awaiting_settings(), obtainable_permits(&state))
}

#[test]
fn given_two_settings_frames_in_one_read_then_both_are_applied_in_order() {
    assert_eq!(
        limit_after_one_read_and_one_more(two_frames(Some(10), Some(0))),
        (false, 0)
    );
    assert_eq!(
        limit_after_one_read_and_one_more(two_frames(Some(0), Some(10))),
        (false, 10)
    );
    assert_eq!(
        limit_after_one_read_and_one_more(two_frames(Some(5), None)),
        (false, 5)
    );
}
