use bumpalo::Bump;
use bumpalo::collections::Vec as BumpVec;
use typst_utils::Id;

/// This is a priority queue based on a binary heap, that allows removing and
/// updating the priority of arbitrary elements.
///
/// This is useful to implement a queue on a collection of elements that can't
/// be mutated directly
pub struct IdQueue<'a, T, F> {
    /// A reverse lookup table that stores the indices of the [Id]s inside
    /// the heap. This is useful when removing or udpating positions of
    /// arbitrary items in the heap.
    ///
    /// Removed items simply have an index larger than the current queue
    /// length.
    reverse_lookup: &'a mut [u32],
    /// A binary heap, used as a priority queue.
    heap: BumpVec<'a, Id<T>>,
    /// The function that priority determines the priority of each item.
    get_priority: F,
}

impl<'a, T, F, P> IdQueue<'a, T, F>
where
    F: Fn(Id<T>) -> P,
    P: Ord,
{
    /// Create a new priority queue from a buffer of [`Id`]s.
    ///
    /// This has worst case complexity of `O(n log(n))`.
    pub fn new(bump: &'a Bump, items: BumpVec<'a, Id<T>>, get_priority: F) -> Self {
        let reverse_lookup =
            bump.alloc_slice_fill_iter((0..items.len()).map(|idx| idx as u32));
        let mut this = Self { reverse_lookup, heap: items, get_priority };
        this.rebuild();
        this
    }

    /// Remove the highest priority item from the queue.
    ///
    /// This has worst case complexity of `O(log(n))`.
    pub fn pop(&mut self) -> Option<Id<T>> {
        if self.heap.is_empty() {
            return None;
        }

        self.swap(0, self.heap.len() - 1);

        let root = self.heap.pop()?;

        if !self.heap.is_empty() {
            self.sift_down(0);
        }

        Some(root)
    }

    /// Remove an arbitrary item from the queue.
    ///
    /// This has worst case complexity of `O(log(n))`.
    pub fn remove(&mut self, item: Id<T>) {
        let idx = self.reverse_lookup[item.idx()] as usize;

        self.swap(idx, self.heap.len() - 1);

        let removed = self.heap.pop().unwrap();
        debug_assert_eq!(item, removed);

        if self.heap.len() == idx {
            // The removed item was the last item, nothing left to do.
            return;
        }

        self.sift_up_or_down(idx);
    }

    /// Notify the queue that the priority of an item changed.
    ///
    /// This has worst case complexity of `O(log(n))`.
    pub fn update(&mut self, item: Id<T>) {
        let idx = self.reverse_lookup[item.idx()] as usize;
        self.sift_up_or_down(idx);
    }

    /// Rebuilds the binary heap structure.
    ///
    /// This has worst case complexity of `O(n log(n))`.
    fn rebuild(&mut self) {
        let mut n = self.heap.len() / 2;
        while n > 0 {
            n -= 1;
            self.sift_down(n);
        }
    }

    /// Either sifts up the item until it's smaller than its parent, or
    /// sifts down the item until it's larger than both of its direct
    /// children, and by extension also all of their children.
    ///
    /// This has worst case complexity of `O(log(n))`.
    fn sift_up_or_down(&mut self, mut node: usize) {
        let mut sifted_up = false;
        while let Some(parent) = parent(node) {
            if self.is_lt(parent, node) {
                self.swap(parent, node);
                node = parent;
                sifted_up = true;
            } else {
                break;
            }
        }

        if sifted_up {
            return;
        }

        self.sift_down(node);
    }

    /// Sifts down the item until it's larger than both of its direct
    /// children, and by extension also all of their children.
    ///
    /// This has worst case complexity of `O(log(n))`.
    fn sift_down(&mut self, mut node: usize) {
        let end = self.heap.len();
        let mut child = first_child(node);

        // While the current node has two children...
        while child <= end.saturating_sub(2) {
            // Compare with the greater of the two children.
            child += self.is_lt(child, child + 1) as usize;

            if !self.is_lt(node, child) {
                // The node is greater than both children, there is nothing
                // left to do.
                return;
            }

            self.swap(node, child);
            node = child;
            child = first_child(node);
        }

        // If `child < end` there is only one child, which is the last
        // element of the heap, otherwise there are none.
        if child < end && self.is_lt(node, child) {
            self.swap(node, child);
        }
    }

    /// # Panics
    /// If the heap indices are out of bounds.
    fn is_lt(&self, a: usize, b: usize) -> bool {
        let a = (self.get_priority)(self.heap[a]);
        let b = (self.get_priority)(self.heap[b]);
        a.lt(&b)
    }

    /// Swaps two ids both in the binary heap and the reverse lookup table.
    fn swap(&mut self, a: usize, b: usize) {
        let id_a = self.heap[a];
        let id_b = self.heap[b];

        self.heap.swap(a, b);
        self.reverse_lookup.swap(id_a.idx(), id_b.idx());
    }
}

impl<T, F, P> Iterator for IdQueue<'_, T, F>
where
    F: Fn(Id<T>) -> P,
    P: Ord,
{
    type Item = Id<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.pop()
    }
}

fn first_child(idx: usize) -> usize {
    2 * idx + 1
}

fn parent(idx: usize) -> Option<usize> {
    if idx == 0 { None } else { Some((idx - 1) / 2) }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use bumpalo::collections::CollectIn;
    use typst_utils::IdVec;

    use super::*;

    struct Item {
        priority: Cell<u128>,
    }

    fn setup<I>(
        bump: &Bump,
        items: I,
    ) -> (&'static IdVec<Item>, IdQueue<'_, Item, impl Fn(Id<Item>) -> u128>)
    where
        I: IntoIterator<Item = u128>,
    {
        let items = items
            .into_iter()
            .map(|p| Item { priority: Cell::new(p) })
            .collect::<IdVec<_>>();
        // HACK: Leak the vector, so we can return the queue with a comparison
        // function that references it.
        let items = Box::leak(Box::new(items));

        let buf = items.ids().collect_in::<BumpVec<_>>(bump);
        let queue = IdQueue::new(bump, buf, |id| items.get(id).priority.get());

        (items, queue)
    }

    fn drain_queue<F>(items: &IdVec<Item>, queue: IdQueue<Item, F>) -> Vec<u128>
    where
        F: Fn(Id<Item>) -> u128,
    {
        queue.map(|id| items.get(id).priority.get()).collect()
    }

    #[test]
    fn simple() {
        let bump = Bump::new();
        let (items, queue) = setup(&bump, [2, 13, 7, 0, 1, 23, 0]);

        let ordered = drain_queue(items, queue);
        assert_eq!(ordered, [23, 13, 7, 2, 1, 0, 0]);
    }

    #[test]
    fn remove_item() {
        let bump = Bump::new();
        let (items, mut queue) = setup(&bump, [2, 13, 7, 10, 5, 23, 0]);

        queue.remove(Id::new(3));

        let ordered = drain_queue(items, queue);
        assert_eq!(ordered, [23, 13, 7, 5, 2, 0]);
    }

    #[test]
    fn remove_multiple() {
        let bump = Bump::new();
        let (items, mut queue) = setup(&bump, [2, 13, 7, 10, 5, 23, 0]);

        queue.remove(Id::new(3)); // 10
        queue.remove(Id::new(6)); // 0

        let ordered = drain_queue(items, queue);
        assert_eq!(ordered, [23, 13, 7, 5, 2]);
    }

    #[test]
    fn pop_and_remove() {
        let bump = Bump::new();
        let (items, mut queue) = setup(&bump, [2, 13, 7, 10, 5, 23, 0]);

        queue.pop(); // 23
        queue.remove(Id::new(6)); // 0
        queue.remove(Id::new(4)); // 5
        queue.remove(Id::new(0)); // 2

        let ordered = drain_queue(items, queue);
        assert_eq!(ordered, [13, 10, 7]);
    }

    #[test]
    fn remove_last_item() {
        let bump = Bump::new();
        let (items, mut queue) = setup(&bump, [2, 13, 7, 10, 5, 23, 0]);

        queue.remove(Id::new(6)); // 0

        let ordered = drain_queue(items, queue);
        assert_eq!(ordered, [23, 13, 10, 7, 5, 2]);
    }

    #[test]
    fn update_with_lower_priority() {
        let bump = Bump::new();
        let (items, mut queue) = setup(&bump, [2, 13, 7, 10, 5, 23, 0]);

        let id_of_seven = Id::new(2);
        items.get(id_of_seven).priority.set(3);
        queue.update(id_of_seven);

        let ordered = drain_queue(items, queue);
        assert_eq!(ordered, [23, 13, 10, 5, 3, 2, 0]);
    }

    #[test]
    fn update_with_higher_priority() {
        let bump = Bump::new();
        let (items, mut queue) = setup(&bump, [2, 13, 7, 10, 5, 23, 0]);

        let id_of_seven = Id::new(2);
        items.get(id_of_seven).priority.set(99);
        queue.update(id_of_seven);

        let ordered = drain_queue(items, queue);
        assert_eq!(ordered, [99, 23, 13, 10, 5, 2, 0]);
    }

    #[test]
    fn update_with_same_priority() {
        let bump = Bump::new();
        let (items, mut queue) = setup(&bump, [2, 13, 7, 10, 5, 23, 0]);

        let id_of_seven = Id::new(2);
        items.get(id_of_seven).priority.set(7);
        queue.update(id_of_seven);

        let ordered = drain_queue(items, queue);
        assert_eq!(ordered, [23, 13, 10, 7, 5, 2, 0]);
    }
}
