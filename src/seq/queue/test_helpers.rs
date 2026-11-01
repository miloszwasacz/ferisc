#[allow(unused)]
pub(super) struct Producer<I>(pub I);

impl<I: Iterator> SimpleSubStage for Producer<I> {
    type In = ();
    type Out = I::Item;
    fn start(&mut self, (): Self::In) -> Option<Self::Out> {
        self.0.next()
    }
    fn finish(&mut self) {}
}

#[allow(unused)]
pub(super) fn init_queue_with<T, I: IntoIterator<Item = T>>(queue: &mut Queue<T>, items: I) {
    Producer(items.into_iter())
        .into_pipeline_stage()
        .substage(queue.push())
        .execute();

    Sequential::finish_cycle(queue);
}
