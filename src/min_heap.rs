pub struct MinHeap<T: MinHeapNode>(Vec<T>);

pub trait MinHeapNode {
    fn frequency(&self) -> u32;
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
    use crate::node::TreeNode;
    use crate::test_cases::*;

    use super::MinHeap;

    #[test]
    fn pop_order() {
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
}

// MARK: Benches

#[divan::bench_group(sample_count = 10_000)]
mod benches_common {
    use super::*;
    use crate::node::TreeNode;
    use crate::packet::Packet;
    use crate::test_cases::{Case, ALL_CASES};

    use divan::counter::BytesCount;
    use divan::{black_box, Bencher};

    #[divan::bench(args = ALL_CASES)]
    fn heap_push(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = &Packet::new(response_bytes);
        let symbol_table = &packet.symbol_table();

        bencher
            .counter(BytesCount::from(symbol_table.len()))
            .bench_local(move || {
                let mut heap = MinHeap::new();
                symbol_table
                    .iter()
                    .for_each(|&(s, f)| heap.push(TreeNode::new(Some(s), f)));
            });
    }

    #[divan::bench(args = ALL_CASES)]
    fn heap_pop(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = &Packet::new(response_bytes);
        let symbol_table = &packet.symbol_table();
        let mut heap = MinHeap::new();
        symbol_table
            .iter()
            .for_each(|&(s, f)| heap.push(TreeNode::new(Some(s), f)));

        bencher
            .counter(BytesCount::from(symbol_table.len()))
            .bench_local(move || {
                while !heap.is_empty() {
                    black_box(heap.pop());
                }
            });
    }
}
