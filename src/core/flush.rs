use crate::core::mem::MemHierarchy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlushInfo(Stage);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
enum Stage {
    #[default]
    None,
    Fetch,
    Decode,
    All,
}

impl FlushInfo {
    pub fn none() -> Self {
        Self(Stage::None)
    }

    pub fn fetch() -> Self {
        Self(Stage::Fetch)
    }

    pub fn decode() -> Self {
        Self(Stage::Decode)
    }

    pub fn commit() -> Self {
        Self(Stage::All)
    }

    pub fn flush_stages<H>(
        self,
        mem: &mut H,
        fetch: impl FnOnce(&mut H),
        decode: impl FnOnce(),
        rest: impl FnOnce(&mut H),
    ) where
        H: MemHierarchy,
    {
        if self.0 >= Stage::Fetch {
            fetch(mem);
        }
        if self.0 >= Stage::Decode {
            decode();
        }
        if self.0 >= Stage::All {
            rest(mem);
        }
    }
}
