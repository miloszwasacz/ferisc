use crate::core::{
    eu::Eu,
    reg::{
        ArchReg,
        int::IntReg,
        phys::{Prf, Rat},
    },
    rs::Rs,
    uop::{
        Infer, InferUop, decode::Uop as DecUop, dispatch::Uop as DUop, issue::Issue,
        rename::Uop as RUop,
    },
};

pub mod locks;

type IUop<T> = Infer<DUop, T>;
type EUop<T> = <IUop<T> as Issue>::Issued;
type Line<T> = (Rs<IUop<T>>, Eu<EUop<T>>);

#[derive(derive_more::Debug)]
pub struct Pipeline<T>
where
    T: ArchReg,
    DUop: InferUop<T>,
    IUop<T>: Issue,
{
    rat: Rat<T>,
    prf: Prf<T>,
    lines: Box<[Line<T>]>,
}

#[derive(Debug)]
pub struct Pipelines {
    int: Pipeline<IntReg>,
}
