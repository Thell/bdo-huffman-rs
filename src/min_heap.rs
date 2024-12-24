pub struct MinHeap<T: MinHeapNode>(Vec<T>);

pub trait MinHeapNode {
    fn frequency(&self) -> u32;
    fn new(symbol: Option<u8>, frequency: u32) -> Self;
}

impl<T: MinHeapNode> Default for MinHeap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: MinHeapNode> MinHeap<T> {
    pub fn new() -> Self {
        MinHeap(Vec::<T>::with_capacity(crate::packet::MAX_SYMBOLS))
    }

    pub fn with_capacity(capacity: usize) -> Self {
        MinHeap(Vec::<T>::with_capacity(capacity))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.len() == 0
    }

    pub fn push(&mut self, node: T) {
        self.0.push(node);
        let mut index = self.0.len() - 1;

        while index > 0 {
            let parent = (index - 1) >> 1;
            if self.0[index].frequency() >= self.0[parent].frequency() {
                break;
            }
            self.0.swap(index, parent);
            index = parent;
        }
    }

    pub fn pop(&mut self) -> T {
        let mut root = self.0.pop().unwrap();
        if self.0.is_empty() {
            return root;
        }

        std::mem::swap(&mut self.0[0], &mut root);
        let mut index = 0;
        let end = self.0.len();

        loop {
            let left = 2 * index + 1;
            if left >= end {
                break;
            }
            let right = left + 1;

            let smallest = if right < end && self.0[right].frequency() < self.0[left].frequency() {
                right
            } else {
                left
            };

            if self.0[smallest].frequency() >= self.0[index].frequency() {
                break;
            }

            self.0.swap(index, smallest);
            index = smallest;
        }

        root
    }
}

// =========================================================

// MARK: Unit Tests

#[cfg(test)]
mod tests {
    use crate::node::{FlatNode, TreeNode};
    use crate::test_cases::*;

    use super::MinHeap;

    #[test]
    fn pop_order_tree() {
        // Tests the integrity of the MinHeap min element ordering.
        let mut heap = MinHeap::<TreeNode>::new();
        for (symbol, frequency) in EXPECTED_SYMBOL_TABLE {
            heap.push(TreeNode::new(Some(symbol), frequency));
        }
        let mut pop_order = Vec::<Option<u8>>::new();
        while !heap.is_empty() {
            pop_order.push(heap.pop().symbol);
        }
        println!("{:?}", pop_order);
        assert_eq!(pop_order, EXPECTED_POP_ORDER);
    }

    #[test]
    fn pop_order_flatnode() {
        // Tests the integrity of the MinHeap min element ordering.
        let mut heap = MinHeap::<FlatNode>::new();
        for (symbol, frequency) in EXPECTED_SYMBOL_TABLE {
            heap.push(FlatNode::new(Some(symbol), frequency));
        }
        let mut pop_order = Vec::<Option<u8>>::new();
        while !heap.is_empty() {
            pop_order.push(heap.pop().symbol);
        }
        println!("{:?}", pop_order);
        assert_eq!(pop_order, EXPECTED_POP_ORDER);
    }
}

// MARK: Benches

#[divan::bench_group(sample_count = 10_000)]
mod common {
    use super::*;
    use crate::node::{FlatNode, TreeNode};
    use crate::test_cases::EXPECTED_SYMBOL_TABLE;

    use divan::{black_box, Bencher};

    #[divan::bench(types = [TreeNode, FlatNode])]
    fn alloc<T: MinHeapNode>() -> MinHeap<T> {
        MinHeap::<T>::default()
    }

    #[divan::bench(types = [TreeNode, FlatNode])]
    fn push<T: MinHeapNode>(bencher: Bencher) {
        let mut heap = MinHeap::<T>::new();
        bencher.bench_local(move || {
            EXPECTED_SYMBOL_TABLE
                .iter()
                .for_each(|&(s, f)| heap.push(T::new(Some(black_box(s)), f)));
        });
    }

    #[divan::bench(types = [TreeNode, FlatNode])]
    fn pop<T: MinHeapNode>(bencher: Bencher) {
        let mut heap = MinHeap::<T>::new();
        EXPECTED_SYMBOL_TABLE
            .iter()
            .for_each(|&(s, f)| heap.push(T::new(Some(s), f)));
        bencher.bench_local(move || {
            while !heap.is_empty() {
                black_box(heap.pop());
            }
        });
    }

    #[divan::bench(types = [TreeNode, FlatNode])]
    fn roundtrip<T: MinHeapNode>() {
        let mut heap = MinHeap::<T>::new();
        EXPECTED_SYMBOL_TABLE
            .iter()
            .for_each(|&(s, f)| heap.push(T::new(Some(s), f)));
        while !heap.is_empty() {
            black_box(heap.pop());
        }
    }
}
