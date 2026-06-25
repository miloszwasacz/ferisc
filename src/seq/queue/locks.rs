use super::Queue;
use crate::seq::locks::{CircuitLock, Transformer};
use std::marker::PhantomData;

//#region Push

pub struct PushLock<'a, T> {
    queue: &'a mut Queue<T>,
    free_space: usize,
}

impl<'a, T> PushLock<'a, T> {
    pub(super) fn new(queue: &'a mut Queue<T>) -> Self {
        let free_space = queue.size - queue.len;
        Self { queue, free_space }
    }
}

impl<'a, T> CircuitLock<'a> for PushLock<'a, T> {
    type Trans = PushTrans<T>;

    fn make_trans(&mut self) -> Self::Trans {
        PushTrans(None)
    }
}

pub struct PushTrans<T>(Option<T>);

impl<'a, T: 'a> Transformer<'a> for PushTrans<T> {
    type In = T;
    type Out = ();
    type Lock = PushLock<'a, T>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        if lock.free_space > 0 {
            self.0 = Some(input);
            Some(())
        } else {
            None
        }
    }

    fn commit(self, lock: &mut Self::Lock) {
        debug_assert!(lock.free_space > 0);
        lock.queue.buf.push_back(self.0.unwrap());
        lock.free_space -= 1;
        #[cfg(debug_assertions)]
        lock.queue.state.insert(super::State::PUSHED);
    }
}

//#endregion

//#region Pop

pub struct PopLock<'a, T>(Option<PopLockImpl<'a, T>>);

struct PopLockImpl<'a, T> {
    queue: *mut Queue<T>,
    iter: std::iter::Take<super::Iter<'a, T>>,
    popped: usize,
    _queue: PhantomData<&'a mut Queue<T>>,
}

impl<'a, T> PopLock<'a, T> {
    pub(super) fn new(queue: &'a mut Queue<T>, max_count: usize) -> Self {
        Self(Some(PopLockImpl {
            queue: queue as _,
            iter: queue.iter().take(max_count),
            popped: 0,
            _queue: PhantomData,
        }))
    }
}

impl<'a, T> CircuitLock<'a> for PopLock<'a, T> {
    type Trans = PopTrans<'a, T>;

    fn make_trans(&mut self) -> Self::Trans {
        PopTrans(self.0.as_mut().unwrap().iter.next())
    }
}

impl<'a, T> Drop for PopLock<'a, T> {
    fn drop(&mut self) {
        // SAFETY: iter is dropped so it is safe to interact with the queue again
        let (queue, popped) = unsafe {
            let PopLockImpl {
                queue,
                iter,
                popped,
                _queue: _,
            } = self.0.take().unwrap();
            #[allow(clippy::drop_non_drop)]
            drop(iter);
            (&mut *queue, popped)
        };

        queue.dealloc = popped;
        #[cfg(debug_assertions)]
        queue.state.insert(super::State::POPPED);
    }
}

pub struct PopTrans<'a, T>(Option<&'a T>);

impl<'a, T> Transformer<'a> for PopTrans<'a, T> {
    type In = ();
    type Out = &'a T;
    type Lock = PopLock<'a, T>;

    fn transform(&mut self, _: &Self::Lock, _: Self::In) -> Option<Self::Out> {
        self.0
    }

    fn commit(self, lock: &mut Self::Lock) {
        lock.0.as_mut().unwrap().popped += 1;
    }
}

//#endregion
