#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Flush {
    #[default]
    None,
    After(usize),
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum FlushPriority {
    Execute,
    Commit,
}
