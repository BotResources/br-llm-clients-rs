use std::collections::{BTreeMap, VecDeque};

pub(crate) const ROOT: u32 = 0;
pub(crate) const MAX_NODES: usize = 1 << 30;

const NONE: u32 = u32::MAX;
const CHAIN: u32 = 1 << 31;
const BRANCH: u32 = 1 << 30;
const FAIL: u32 = BRANCH - 1;

pub(crate) struct Automaton {
    bytes: Vec<u8>,
    links: Vec<u32>,
    longest: Vec<u32>,
    root: Vec<u32>,
    branches: BTreeMap<(u32, u8), u32>,
    starts: Vec<(u32, u32)>,
}

fn slot(index: u32) -> usize {
    usize::try_from(index).unwrap_or(usize::MAX)
}

impl Automaton {
    pub(crate) fn new(secrets: &[&[u8]], max_nodes: usize) -> Option<Self> {
        let total = secrets
            .iter()
            .fold(0_usize, |total, secret| total.saturating_add(secret.len()));
        let nodes = total.saturating_add(1);
        if nodes > max_nodes.min(MAX_NODES) {
            return None;
        }
        let mut automaton = Self {
            bytes: Vec::with_capacity(nodes),
            links: Vec::with_capacity(nodes),
            longest: Vec::with_capacity(nodes),
            root: vec![NONE; 256],
            branches: BTreeMap::new(),
            starts: Vec::new(),
        };
        automaton.push_node(0)?;
        for secret in secrets {
            automaton.insert(secret)?;
        }
        automaton.link();
        Some(automaton)
    }

    pub(crate) fn step(&self, state: u32, byte: u8) -> u32 {
        let mut state = state;
        loop {
            if let Some(next) = self.child(state, byte) {
                return next;
            }
            if state == ROOT {
                return ROOT;
            }
            state = self.fail(state);
        }
    }

    pub(crate) fn longest_ending(&self, state: u32) -> usize {
        slot(self.longest_of(state))
    }

    pub(crate) fn open_depth(&self, state: u32) -> usize {
        let mut state = state;
        loop {
            if self.has_child(state) {
                return self.depth(state);
            }
            if state == ROOT {
                return 0;
            }
            state = self.fail(state);
        }
    }

    #[cfg(test)]
    pub(crate) fn storage_bytes(&self) -> usize {
        self.bytes.capacity()
            + self.links.capacity() * size_of::<u32>()
            + self.longest.capacity() * size_of::<u32>()
            + self.root.capacity() * size_of::<u32>()
            + self.branches.len() * size_of::<((u32, u8), u32)>()
            + self.starts.capacity() * size_of::<(u32, u32)>()
    }

    fn longest_of(&self, node: u32) -> u32 {
        self.longest.get(slot(node)).copied().unwrap_or(0)
    }

    fn link_of(&self, node: u32) -> u32 {
        self.links.get(slot(node)).copied().unwrap_or(ROOT)
    }

    fn fail(&self, node: u32) -> u32 {
        self.link_of(node) & FAIL
    }

    fn has_child(&self, node: u32) -> bool {
        if node == ROOT {
            return self.root.iter().any(|child| *child != NONE);
        }
        self.link_of(node) & (CHAIN | BRANCH) != 0
    }

    fn depth(&self, node: u32) -> usize {
        let before = self.starts.partition_point(|(first, _)| *first <= node);
        match before.checked_sub(1).and_then(|at| self.starts.get(at)) {
            Some((first, depth)) => slot(depth.saturating_add(node.saturating_sub(*first))),
            None => 0,
        }
    }

    fn child(&self, node: u32, byte: u8) -> Option<u32> {
        if node == ROOT {
            return self
                .root
                .get(usize::from(byte))
                .copied()
                .filter(|child| *child != NONE);
        }
        let link = self.link_of(node);
        let next = node.saturating_add(1);
        if link & CHAIN != 0 && self.bytes.get(slot(next)) == Some(&byte) {
            return Some(next);
        }
        if link & BRANCH != 0 {
            return self.branches.get(&(node, byte)).copied();
        }
        None
    }

    fn push_node(&mut self, byte: u8) -> Option<u32> {
        let index = u32::try_from(self.bytes.len())
            .ok()
            .filter(|index| *index <= FAIL)?;
        self.bytes.push(byte);
        self.links.push(ROOT);
        self.longest.push(0);
        Some(index)
    }

    fn mark(&mut self, node: u32, flag: u32) {
        if let Some(link) = self.links.get_mut(slot(node)) {
            *link |= flag;
        }
    }

    fn add_child(&mut self, parent: u32, byte: u8) -> Option<u32> {
        let index = self.push_node(byte)?;
        if parent == ROOT {
            if let Some(entry) = self.root.get_mut(usize::from(byte)) {
                *entry = index;
            }
        } else if parent.checked_add(1) == Some(index) {
            self.mark(parent, CHAIN);
        } else {
            self.branches.insert((parent, byte), index);
            self.mark(parent, BRANCH);
        }
        Some(index)
    }

    fn insert(&mut self, secret: &[u8]) -> Option<()> {
        let mut node = ROOT;
        let mut fresh = false;
        for (depth, byte) in (1_u32..).zip(secret) {
            let existing = if fresh { None } else { self.child(node, *byte) };
            node = match existing {
                Some(next) => next,
                None => {
                    let created = self.add_child(node, *byte)?;
                    if !fresh {
                        self.starts.push((created, depth));
                        fresh = true;
                    }
                    created
                }
            };
        }
        let length = u32::try_from(secret.len()).ok()?;
        if let Some(longest) = self.longest.get_mut(slot(node)).filter(|_| node != ROOT) {
            *longest = length;
        }
        Some(())
    }

    fn link(&mut self) {
        let mut queue: VecDeque<u32> = self
            .root
            .iter()
            .copied()
            .filter(|child| *child != NONE)
            .collect();
        while let Some(node) = queue.pop_front() {
            let fail = self.fail(node);
            let inherited = self.longest_of(fail);
            if let Some(longest) = self.longest.get_mut(slot(node))
                && *longest == 0
            {
                *longest = inherited;
            }
            let link = self.link_of(node);
            if link & CHAIN != 0 {
                self.link_child(node.saturating_add(1), fail, &mut queue);
            }
            if link & BRANCH != 0 {
                let branches: Vec<u32> = self
                    .branches
                    .range((node, 0)..=(node, u8::MAX))
                    .map(|(_, child)| *child)
                    .collect();
                for child in branches {
                    self.link_child(child, fail, &mut queue);
                }
            }
        }
    }

    fn link_child(&mut self, child: u32, parent_fail: u32, queue: &mut VecDeque<u32>) {
        let byte = self.bytes.get(slot(child)).copied().unwrap_or(0);
        let target = self.step(parent_fail, byte);
        if let Some(link) = self.links.get_mut(slot(child)) {
            *link = (*link & !FAIL) | (target & FAIL);
        }
        queue.push_back(child);
    }
}
