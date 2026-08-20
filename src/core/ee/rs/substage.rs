pub use self::dispatch::Dispatch;
pub use self::issue::Issue;
use super::Rs;
use crate::seq::{Outcome, Substage};
use std::convert::Infallible;

mod dispatch {
    use super::*;
    use crate::core::ee::rs::RsEntry;
    use std::collections::VecDeque;

    pub struct Dispatch<'a, U> {
        new: &'a mut VecDeque<RsEntry<U>>,
        max_size: usize,
    }

    impl<'a, U> Dispatch<'a, U> {
        pub(in crate::core::ee::rs) fn new(rs: &'a mut Rs<U>) -> Self {
            debug_assert!(
                rs.state.pushed.is_empty(),
                "cannot dispatch more than once per clock cycle",
            );

            //TODO More than one issue per clock cycle?
            let issued = if rs.state.removed.is_some() { 1 } else { 0 };
            Self {
                new: &mut rs.state.pushed,
                max_size: rs.size.get() - rs.entries.len() + issued,
            }
        }

        pub fn free_space(&self) -> usize {
            self.max_size - self.new.len()
        }
    }

    impl<'a, U> Substage for Dispatch<'a, U> {
        type In = U;
        type Out = ();
        type Extra = U;
        type Err = Infallible;

        fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
            //TODO Make RSs operation-specific (ALU, AGU, BR, etc.)
            if self.free_space() > 0 {
                Outcome::Ok((), input)
            } else {
                Outcome::Stop
            }
        }

        fn finish(&mut self, extra: Self::Extra) {
            self.new.push_back(extra.into());
            debug_assert!(self.new.len() <= self.max_size);
        }
    }
}

mod issue {
    use super::*;
    use crate::core::{ee::rs::Uop, uop::issue::TryReadReg};

    pub struct Issue<'a, U: Uop, R>(Option<Inner<'a, U, R>>);

    struct Inner<'a, U: Uop, R> {
        rs: &'a mut Rs<U>,
        read: R,
    }

    impl<'a, U: Uop, R: TryReadReg<U::Reg>> Issue<'a, U, R> {
        pub(in crate::core::ee::rs) fn new(rs: &'a mut Rs<U>, read: R) -> Self {
            debug_assert!(
                rs.state.removed.is_none(),
                "cannot issue more than once per clock cycle"
            );

            Self(Some(Inner { rs, read }))
        }
    }

    impl<'a, U: Uop, R: TryReadReg<U::Reg> + Clone> Substage for Issue<'a, U, R> {
        type In = ();
        type Out = U::Issued;
        type Extra = (&'a mut Rs<U>, usize);
        type Err = Infallible;

        //TODO More than one issue per clock cycle?
        fn start(&mut self, (): Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
            let Some(Inner { rs, read }) = self.0.take() else {
                return Outcome::Stop;
            };

            rs.entries
                .iter()
                .enumerate()
                .find_map(|(i, entry)| entry.issue(read.clone()).map(|issued| (issued, i)))
                .map(|(issued, i)| Outcome::Ok(issued, (rs, i)))
                .unwrap_or(Outcome::Stop)
        }

        fn finish(&mut self, extra: Self::Extra) {
            let (rs, removed) = extra;
            let prev = rs.state.removed.replace(removed);
            debug_assert!(prev.is_none())
        }
    }
}
