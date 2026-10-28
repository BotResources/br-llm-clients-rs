use std::collections::VecDeque;

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
    branches: Vec<(u32, u8, u32)>,
    starts: Vec<(u32, u32)>,
}

fn slot(index: u32) -> usize {
    usize::try_from(index).unwrap_or(usize::MAX)
}

pub(crate) fn reserved<T>(length: usize) -> Option<Vec<T>> {
    let mut reserved = Vec::new();
    reserved.try_reserve_exact(length).ok()?;
    Some(reserved)
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
            bytes: reserved(nodes)?,
            links: reserved(nodes)?,
            longest: reserved(nodes)?,
            root: vec![NONE; 256],
            branches: reserved(secrets.len())?,
            starts: reserved(secrets.len())?,
        };
        automaton.push_node(0)?;
        let mut sorted = reserved(secrets.len())?;
        sorted.extend_from_slice(secrets);
        sorted.sort_unstable();
        let deepest = secrets.iter().map(|secret| secret.len()).max().unwrap_or(0);
        let mut path = reserved(deepest.saturating_add(1))?;
        path.push(ROOT);
        let mut previous: &[u8] = &[];
        for secret in sorted {
            automaton.insert(secret, previous, &mut path)?;
            previous = secret;
        }
        automaton
            .branches
            .sort_unstable_by_key(|(parent, byte, _)| (*parent, *byte));
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
    pub(crate) fn capacities(&self) -> [usize; 5] {
        [
            self.bytes.capacity(),
            self.links.capacity(),
            self.longest.capacity(),
            self.branches.capacity(),
            self.starts.capacity(),
        ]
    }

    #[cfg(test)]
    pub(crate) fn storage_bytes(&self) -> usize {
        self.bytes.capacity()
            + self.links.capacity() * size_of::<u32>()
            + self.longest.capacity() * size_of::<u32>()
            + self.root.capacity() * size_of::<u32>()
            + self.branches.capacity() * size_of::<(u32, u8, u32)>()
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
            return self
                .branches
                .binary_search_by_key(&(node, byte), |(parent, byte, _)| (*parent, *byte))
                .ok()
                .and_then(|at| self.branches.get(at))
                .map(|(_, _, child)| *child);
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
            self.branches.push((parent, byte, index));
            self.mark(parent, BRANCH);
        }
        Some(index)
    }

    fn insert(&mut self, secret: &[u8], previous: &[u8], path: &mut Vec<u32>) -> Option<()> {
        let shared = previous
            .iter()
            .zip(secret)
            .take_while(|(before, now)| before == now)
            .count();
        path.truncate(shared.saturating_add(1));
        let mut node = *path.last()?;
        let first_depth = u32::try_from(shared).ok()?.checked_add(1)?;
        for (depth, byte) in (first_depth..).zip(secret.get(shared..)?) {
            node = self.add_child(node, *byte)?;
            if depth == first_depth {
                self.starts.push((node, depth));
            }
            path.push(node);
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
                let from = self
                    .branches
                    .partition_point(|(parent, _, _)| *parent < node);
                let to = self
                    .branches
                    .partition_point(|(parent, _, _)| *parent <= node);
                let branches: Vec<u32> = self
                    .branches
                    .get(from..to)
                    .unwrap_or_default()
                    .iter()
                    .map(|(_, _, child)| *child)
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
