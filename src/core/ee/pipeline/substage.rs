pub use self::commit::Commit;
pub use self::rename_dispatch::{Dispatch, Rename};
use super::{DUop, EUop, IUop, Pipeline};
use crate::core::{
    csr::CsrFile,
    ee::{eu, rob, rs},
    reg::{ArchReg, phys::rat},
    uop,
    uop::InferUop,
};
use crate::seq::{Outcome, Substage};
use std::convert::Infallible;

mod rename_dispatch {
    use super::*;
    use itertools::Itertools;

    pub type Rename<'a, T> = rat::substage::Rename<'a, T>;

    pub struct Dispatch<'a, T>(Box<[rs::substage::Dispatch<'a, IUop<T>>]>)
    where
        T: ArchReg,
        DUop: InferUop<T>,
        IUop<T>: rs::Uop<Reg = T> + Clone,
        EUop<T>: eu::Uop<Reg = T>;

    impl<T> Pipeline<T>
    where
        T: ArchReg,
        DUop: InferUop<T>,
        IUop<T>: rs::Uop<Reg = T> + Clone,
        EUop<T>: eu::Uop<Reg = T>,
    {
        pub fn rename_and_dispatch(&mut self) -> (Rename<'_, T>, Dispatch<'_, T>) {
            let lines = self
                .lines
                .iter_mut()
                .map(|(rs, _)| rs.dispatch())
                .sorted_by_key(|rs| std::cmp::Reverse(rs.free_space()))
                .collect::<Vec<_>>()
                .into_boxed_slice();

            (self.rat.rename(&mut self.prf), Dispatch(lines))
        }
    }

    impl<'a, T> Substage for Dispatch<'a, T>
    where
        T: ArchReg,
        DUop: InferUop<T>,
        IUop<T>: rs::Uop<Reg = T> + Clone,
        EUop<T>: eu::Uop<Reg = T>,
    {
        type In = IUop<T>;
        type Out = ();
        type Extra = (usize, IUop<T>);
        type Err = Infallible;

        fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
            self.0
                .iter_mut()
                .enumerate()
                .find_map(|(i, dispatch)| match dispatch.start(input.clone()) {
                    Outcome::Ok((), meta) => Some((i, meta)),
                    Outcome::Stop => None,
                    Outcome::Err(err) => match err {},
                })
                .map(|extra| Outcome::Ok((), extra))
                .unwrap_or(Outcome::Stop)
        }

        fn finish(&mut self, (i, extra): Self::Extra) {
            self.0[i].finish(extra);

            // Bubble up the modified dispatcher so that the list remains sorted
            for [l, r] in self.0[i..].array_windows() {
                // SAFETY: Due to limitations of the borrow checker, there is no
                //         `windows_mut` iterator, but these references are
                //         guaranteed to be disjoint.
                let l = unsafe {
                    (l as *const rs::substage::Dispatch<'_, IUop<T>>)
                        .cast_mut()
                        .as_mut_unchecked()
                };
                let r = unsafe {
                    (r as *const rs::substage::Dispatch<'_, IUop<T>>)
                        .cast_mut()
                        .as_mut_unchecked()
                };

                if l.free_space() < r.free_space() {
                    std::mem::swap(l, r);
                } else {
                    break;
                }
            }
        }
    }
}

mod issue_execute {
    use super::*;
    use crate::seq::IntoPipelineStage;

    impl<T> Pipeline<T>
    where
        T: ArchReg,
        DUop: InferUop<T>,
        IUop<T>: rs::Uop<Reg = T>,
        EUop<T>: eu::Uop<Reg = T>,
    {
        pub fn issue(&mut self, rob: &mut rob::lock::Execute<'_, '_>, csrf: &mut CsrFile)
        where
            T::Repr: Clone,
        {
            let mut written = Vec::new();
            for (rs, eu) in &mut self.lines {
                let read_reg = |reg| self.prf[reg].try_read_bypassed().cloned();
                let write_reg = |reg, val| written.push((reg, val));

                rs.issue(read_reg)
                    .into_pipeline_stage()
                    .substage(eu.issue(Jump(rob), write_reg, csrf))
                    .execute();
            }
            for (reg, val) in written {
                self.prf[reg].write(val);
            }
        }

        pub fn execute(&mut self, rob: &mut rob::lock::Execute<'_, '_>, csrf: &mut CsrFile)
        where
            EUop<T>: Clone,
        {
            for (_, eu) in &mut self.lines {
                eu.execute(Jump(rob), |reg, val| self.prf[reg].write(val), csrf);
            }
        }
    }

    //TODO Replace with a closure once `trait Jump` becomes `type Jump = impl FnOnce(..)`
    struct Jump<'a, 'b, 'c>(&'a mut rob::lock::Execute<'b, 'c>);
    impl<'a, 'b, 'c> uop::execute::Jump for Jump<'a, 'b, 'c> {
        fn jump(self, target: crate::mem::Addr, cause: &rob::Handle) {
            self.0.jump(target, cause)
        }
    }
}

mod commit {
    use super::*;

    pub type Commit<'a, T> = rat::substage::Commit<'a, T>;

    impl<T> Pipeline<T>
    where
        T: ArchReg,
        DUop: InferUop<T>,
        IUop<T>: rs::Uop<Reg = T>,
        EUop<T>: eu::Uop<Reg = T>,
    {
        pub fn commit(&mut self) -> Commit<'_, T> {
            self.rat.commit(&mut self.prf)
        }
    }
}
