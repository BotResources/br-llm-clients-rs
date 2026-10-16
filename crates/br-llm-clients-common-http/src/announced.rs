#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Announced {
    Awaiting,
    ZeroSinceFirst,
    Usable(usize),
}

impl Announced {
    pub(crate) fn limit(self) -> usize {
        match self {
            Self::Usable(limit) => limit,
            Self::Awaiting | Self::ZeroSinceFirst => 0,
        }
    }

    pub(crate) fn is_awaiting(self) -> bool {
        self == Self::Awaiting
    }

    pub(crate) fn is_zero_since_first(self) -> bool {
        self == Self::ZeroSinceFirst
    }

    pub(crate) fn after_run(self, run: &[Option<u32>]) -> Option<Self> {
        let mut next = self;
        let mut changed = false;
        for entry in run {
            let limit = match entry {
                Some(value) => usize::try_from(*value).unwrap_or(usize::MAX),
                None if next.is_awaiting() => usize::MAX,
                None => continue,
            };
            next = next.after(limit);
            changed = true;
        }
        changed.then_some(next)
    }

    fn after(self, limit: usize) -> Self {
        match self {
            Self::Awaiting | Self::ZeroSinceFirst if limit == 0 => Self::ZeroSinceFirst,
            Self::Awaiting | Self::ZeroSinceFirst | Self::Usable(_) => Self::Usable(limit),
        }
    }
}

#[cfg(test)]
#[path = "announced_tests.rs"]
mod tests;
