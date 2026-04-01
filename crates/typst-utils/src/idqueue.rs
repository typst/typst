use crate::Id;

/// A priority queue based on a binary heap, that allows removing and updating
/// the priority of arbitrary elements.
///
/// This is useful to implement a queue on a collection of elements that can't
/// be mutated directly.
pub struct IdQueue<T, F> {
    /// A binary heap, used as a priority queue.
    heap: Vec<Id<T>>,
    /// A reverse lookup table that stores the indices of the [`Id`]s inside the
    /// binary heap. This is useful when removing or udpating positions of
    /// arbitrary items in the heap.
    ///
    /// Removed items simply have an index larger than the current queue length.
    reverse_lookup: Vec<u32>,
    /// The function that priority determines the priority of each item.
    get_priority: F,
}

impl<T, F, P> IdQueue<T, F>
where
    F: Fn(Id<T>) -> P,
    P: Ord,
{
    /// Create a new priority queue from a buffer of [`Id`]s.
    ///
    /// This has worst case complexity of `O(n log(n))`.
    pub fn new(ids: Vec<Id<T>>, get_priority: F) -> Self {
        // Build the reverse lookup table. If the list of IDs is sparse, the
        // reverse lookup table will be larger than the binary heap.
        let len = ids.iter().map(|id| id.idx() + 1).max().unwrap_or(0);
        let mut reverse_lookup = vec![0; len];
        for (i, id) in ids.iter().enumerate() {
            reverse_lookup[id.idx()] = i as u32;
        }

        let mut this = Self { heap: ids, reverse_lookup, get_priority };
        this.rebuild();
        this
    }

    /// Return the length of the queue.
    pub fn len(&self) -> usize {
        self.heap.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Remove the highest priority item from the queue.
    ///
    /// This has worst case complexity of `O(log(n))`.
    pub fn pop(&mut self) -> Option<Id<T>> {
        if self.is_empty() {
            return None;
        }

        self.swap(0, self.heap.len() - 1);

        let root = self.heap.pop()?;

        if !self.is_empty() {
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

        if self.len() == idx {
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

    /// Create an iterator that drains this queue in priority order.
    pub fn drain(self) -> Drain<T, F> {
        Drain(self)
    }

    /// Returns the backing slice of the binary heap.
    ///
    /// The first item is the item with the highest priority, the order of all
    /// other items can't be cheaply computed without draining the queue.
    pub fn as_slice(&self) -> &[Id<T>] {
        &self.heap[..self.heap.len()]
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

    /// Either sifts up the item until it's smaller than its parent, or sifts
    /// down the item until it's larger than both of its direct children, and by
    /// extension also all of their children.
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

    /// Sifts down the item until it's larger than both of its direct children,
    /// and by extension also all of their children.
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
                // The node is greater than both children, there is nothing left
                // to do.
                return;
            }

            self.swap(node, child);
            node = child;
            child = first_child(node);
        }

        // If `child < end` there is only one child, which is the last element
        // of the heap, otherwise there are none.
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
    #[cfg_attr(debug_assertions, track_caller)]
    fn swap(&mut self, a: usize, b: usize) {
        let id_a = self.heap[a];
        let id_b = self.heap[b];

        self.heap.swap(a, b);
        self.reverse_lookup.swap(id_a.idx(), id_b.idx());
    }
}

impl<T, F, P> IntoIterator for IdQueue<T, F>
where
    F: Fn(Id<T>) -> P,
    P: Ord,
{
    type Item = Id<T>;

    type IntoIter = Drain<T, F>;

    fn into_iter(self) -> Self::IntoIter {
        Drain(self)
    }
}

/// An iterator that drains the priority queue by continuously calling
/// [`IdQueue::pop`].
pub struct Drain<T, F>(IdQueue<T, F>);

impl<T, F, P> Iterator for Drain<T, F>
where
    F: Fn(Id<T>) -> P,
    P: Ord,
{
    type Item = Id<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
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

    use crate::IdVec;

    use super::*;

    type TestQueue<F> = IdQueue<Item, F>;

    struct Item {
        priority: Cell<u128>,
    }

    fn setup<I>(items: I) -> (&'static IdVec<Item>, TestQueue<impl Fn(Id<Item>) -> u128>)
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

        let buf = items.ids().collect::<Vec<_>>();
        let queue = IdQueue::new(buf, |id| items.get(id).priority.get());

        (items, queue)
    }

    fn drain_queue<F>(items: &IdVec<Item>, queue: TestQueue<F>) -> Vec<u128>
    where
        F: Fn(Id<Item>) -> u128,
    {
        queue.drain().map(|id| items.get(id).priority.get()).collect()
    }

    #[test]
    fn simple() {
        let (items, queue) = setup([2, 13, 7, 0, 1, 23, 0]);

        let ordered = drain_queue(items, queue);
        assert_eq!(ordered, [23, 13, 7, 2, 1, 0, 0]);
    }

    #[test]
    fn remove_item() {
        let (items, mut queue) = setup([2, 13, 7, 10, 5, 23, 0]);

        queue.remove(Id::new(3));

        let ordered = drain_queue(items, queue);
        assert_eq!(ordered, [23, 13, 7, 5, 2, 0]);
    }

    #[test]
    fn remove_multiple() {
        let (items, mut queue) = setup([2, 13, 7, 10, 5, 23, 0]);

        queue.remove(Id::new(3)); // 10
        queue.remove(Id::new(6)); // 0

        let ordered = drain_queue(items, queue);
        assert_eq!(ordered, [23, 13, 7, 5, 2]);
    }

    #[test]
    fn pop_and_remove() {
        let (items, mut queue) = setup([2, 13, 7, 10, 5, 23, 0]);

        queue.pop(); // 23
        queue.remove(Id::new(6)); // 0
        queue.remove(Id::new(4)); // 5
        queue.remove(Id::new(0)); // 2

        let ordered = drain_queue(items, queue);
        assert_eq!(ordered, [13, 10, 7]);
    }

    #[test]
    fn remove_last_item() {
        let (items, mut queue) = setup([2, 13, 7, 10, 5, 23, 0]);

        queue.remove(Id::new(6)); // 0

        let ordered = drain_queue(items, queue);
        assert_eq!(ordered, [23, 13, 10, 7, 5, 2]);
    }

    #[test]
    fn update_with_lower_priority() {
        let (items, mut queue) = setup([2, 13, 7, 10, 5, 23, 0]);

        let id_of_seven = Id::new(2);
        items.get(id_of_seven).priority.set(3);
        queue.update(id_of_seven);

        let ordered = drain_queue(items, queue);
        assert_eq!(ordered, [23, 13, 10, 5, 3, 2, 0]);
    }

    #[test]
    fn update_with_higher_priority() {
        let (items, mut queue) = setup([2, 13, 7, 10, 5, 23, 0]);

        let id_of_seven = Id::new(2);
        items.get(id_of_seven).priority.set(99);
        queue.update(id_of_seven);

        let ordered = drain_queue(items, queue);
        assert_eq!(ordered, [99, 23, 13, 10, 5, 2, 0]);
    }

    #[test]
    fn update_with_same_priority() {
        let (items, mut queue) = setup([2, 13, 7, 10, 5, 23, 0]);

        let id_of_seven = Id::new(2);
        items.get(id_of_seven).priority.set(7);
        queue.update(id_of_seven);

        let ordered = drain_queue(items, queue);
        assert_eq!(ordered, [23, 13, 10, 7, 5, 2, 0]);
    }
}
