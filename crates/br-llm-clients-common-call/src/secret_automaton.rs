use std::collections::VecDeque;

pub(crate) const ROOT: usize = 0;
const NONE: usize = usize::MAX;

struct Node {
    first_child: usize,
    sibling: usize,
    byte: u8,
    fail: usize,
    depth: usize,
    longest: usize,
    whole: bool,
}

impl Node {
    fn new(byte: u8, depth: usize, sibling: usize) -> Self {
        Self {
            first_child: NONE,
            sibling,
            byte,
            fail: ROOT,
            depth,
            longest: 0,
            whole: false,
        }
    }
}

pub(crate) struct Automaton {
    nodes: Vec<Node>,
    root_children: Vec<usize>,
}

impl Automaton {
    pub(crate) fn new<'a>(secrets: impl Iterator<Item = &'a [u8]>) -> Self {
        let mut automaton = Self {
            nodes: vec![Node::new(0, 0, NONE)],
            root_children: vec![NONE; 256],
        };
        for secret in secrets {
            automaton.insert(secret);
        }
        automaton.link();
        automaton
    }

    pub(crate) fn step(&self, state: usize, byte: u8) -> usize {
        let mut state = state;
        loop {
            if let Some(next) = self.child(state, byte) {
                return next;
            }
            if state == ROOT {
                return ROOT;
            }
            state = self.nodes.get(state).map_or(ROOT, |node| node.fail);
        }
    }

    pub(crate) fn longest_ending(&self, state: usize) -> usize {
        self.nodes.get(state).map_or(0, |node| node.longest)
    }

    pub(crate) fn open_depth(&self, state: usize) -> usize {
        let mut state = state;
        while let Some(node) = self.nodes.get(state) {
            if node.first_child != NONE {
                return node.depth;
            }
            if state == ROOT {
                break;
            }
            state = node.fail;
        }
        0
    }

    fn child(&self, node: usize, byte: u8) -> Option<usize> {
        if node == ROOT {
            return self
                .root_children
                .get(usize::from(byte))
                .copied()
                .filter(|child| *child != NONE);
        }
        let mut at = self.nodes.get(node)?.first_child;
        while let Some(current) = self.nodes.get(at) {
            if current.byte == byte {
                return Some(at);
            }
            at = current.sibling;
        }
        None
    }

    fn insert(&mut self, secret: &[u8]) {
        let mut node = ROOT;
        for (depth, byte) in (1_usize..).zip(secret) {
            node = match self.child(node, *byte) {
                Some(next) => next,
                None => self.add_child(node, *byte, depth),
            };
        }
        if let Some(end) = self.nodes.get_mut(node).filter(|_| node != ROOT) {
            end.whole = true;
        }
    }

    fn add_child(&mut self, parent: usize, byte: u8, depth: usize) -> usize {
        let index = self.nodes.len();
        let sibling = self.first_child(parent);
        self.nodes.push(Node::new(byte, depth, sibling));
        if let Some(node) = self.nodes.get_mut(parent) {
            node.first_child = index;
        }
        if parent == ROOT
            && let Some(slot) = self.root_children.get_mut(usize::from(byte))
        {
            *slot = index;
        }
        index
    }

    fn first_child(&self, node: usize) -> usize {
        self.nodes.get(node).map_or(NONE, |node| node.first_child)
    }

    fn link(&mut self) {
        let mut queue = VecDeque::new();
        self.link_children(ROOT, ROOT, &mut queue);
        while let Some(node) = queue.pop_front() {
            let (fail, whole, depth) = match self.nodes.get(node) {
                Some(current) => (current.fail, current.whole, current.depth),
                None => continue,
            };
            let longest = if whole {
                depth
            } else {
                self.longest_ending(fail)
            };
            if let Some(current) = self.nodes.get_mut(node) {
                current.longest = longest;
            }
            self.link_children(node, fail, &mut queue);
        }
    }

    fn link_children(&mut self, node: usize, fail: usize, queue: &mut VecDeque<usize>) {
        let mut child = self.first_child(node);
        while let Some(current) = self.nodes.get(child) {
            let (byte, sibling) = (current.byte, current.sibling);
            let target = if node == ROOT {
                ROOT
            } else {
                self.step(fail, byte)
            };
            if let Some(current) = self.nodes.get_mut(child) {
                current.fail = target;
            }
            queue.push_back(child);
            child = sibling;
        }
    }
}
