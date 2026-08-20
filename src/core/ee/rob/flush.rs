use crate::core::ee::rob;

#[derive(Debug, Clone, Default)]
pub(super) enum Flush {
    #[default]
    None,
    Partial(rob::Handle),
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum FlushPriority {
    Execute,
    Commit,
    Flush,
}
