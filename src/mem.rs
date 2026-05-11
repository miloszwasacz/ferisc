/// Physical address in memory.
pub type Addr = u64;
const _: () = {
    assert!(
        size_of::<Addr>() <= size_of::<usize>(),
        "address must fit in usize"
    );
};
