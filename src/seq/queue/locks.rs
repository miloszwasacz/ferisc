use super::Queue;
use crate::seq::{CircuitLock, Transformer};
use std::collections::VecDeque;
use std::marker::PhantomData;

type Iter<'a, T> = std::iter::Take<super::Iter<'a, T>>;

//#region Push

pub struct QueuePushLock<'a, T> {
    queue: &'a mut Queue<T>,
    pushed: VecDeque<T>,
}

impl<'a, T> QueuePushLock<'a, T> {
    pub(super) fn new(queue: &'a mut Queue<T>) -> Self {
        Self {
            queue,
            pushed: VecDeque::new(),
        }
    }

    fn free_space(&self) -> usize {
        self.queue.size - self.queue.len - self.pushed.len()
    }
}

impl<'a, T> CircuitLock<'a> for QueuePushLock<'a, T> {
    type Trans = PushTrans<T>;

    fn make_trans(&mut self) -> Self::Trans {
        PushTrans(None)
    }
}

impl<'a, T> Drop for QueuePushLock<'a, T> {
    fn drop(&mut self) {
        debug_assert!(self.free_space() >= self.pushed.len());
        self.queue.buf.append(&mut self.pushed);
        #[cfg(debug_assertions)]
        self.queue.state.insert(super::State::PUSHED);
    }
}

pub struct PushTrans<T>(Option<T>);

impl<'a, T: 'a> Transformer<'a> for PushTrans<T> {
    type In = T;
    type Out = ();
    type Lock = QueuePushLock<'a, T>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        if lock.free_space() != 0 {
            self.0 = Some(input);
            Some(())
        } else {
            None
        }
    }

    fn commit(mut self, lock: &mut Self::Lock) {
        lock.pushed.push_back(self.0.take().unwrap());
    }
}

//#endregion

//#region Pop

pub struct QueuePopLock<'a, T>(Option<PopLock<'a, T>>);

struct PopLock<'a, T> {
    queue: *mut Queue<T>,
    iter: Iter<'a, T>,
    popped: usize,
    _queue: PhantomData<&'a mut Queue<T>>,
}

impl<'a, T> QueuePopLock<'a, T> {
    pub(super) fn new(queue: &'a mut Queue<T>, max_count: usize) -> Self {
        Self(Some(PopLock {
            queue: queue as _,
            iter: queue.iter().take(max_count),
            popped: 0,
            _queue: PhantomData,
        }))
    }
}

impl<'a, T> CircuitLock<'a> for QueuePopLock<'a, T> {
    type Trans = PopTrans<'a, T>;

    fn make_trans(&mut self) -> Self::Trans {
        PopTrans(self.0.as_mut().unwrap().iter.next())
    }
}

impl<'a, T> Drop for QueuePopLock<'a, T> {
    fn drop(&mut self) {
        // SAFETY: iter is dropped so it is safe to interact with the queue again
        let (queue, popped) = unsafe {
            let PopLock {
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
    type Lock = QueuePopLock<'a, T>;

    fn transform(&mut self, _: &Self::Lock, _: Self::In) -> Option<Self::Out> {
        self.0
    }

    fn commit(self, lock: &mut Self::Lock) {
        lock.0.as_mut().unwrap().popped += 1;
    }
}

//#endregion
