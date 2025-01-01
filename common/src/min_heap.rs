use crate::packet::MAX_SYMBOLS;

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
        MinHeap(Vec::<T>::with_capacity(MAX_SYMBOLS))
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

// MARK: Unit Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_cases::*;

    #[repr(C)]
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct HeapNode {
        pub left_index: u8,
        pub right_index: u8,
        pub symbol: Option<u8>,
        pub frequency: u32,
    }

    impl MinHeapNode for HeapNode {
        fn frequency(&self) -> u32 {
            self.frequency
        }
    }

    impl Ord for HeapNode {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.frequency.cmp(&other.frequency)
        }
    }
    impl PartialOrd for HeapNode {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl HeapNode {
        pub fn new(symbol: Option<u8>, frequency: u32) -> Self {
            Self {
                symbol,
                frequency,
                left_index: 0,
                right_index: 0,
            }
        }
    }

    #[test]
    fn pop_order() {
        // Tests the integrity of the MinHeap min element ordering.
        let mut heap = MinHeap::<HeapNode>::new();
        for (symbol, frequency) in EXPECTED_SYMBOL_TABLE {
            heap.push(HeapNode::new(Some(symbol), frequency));
        }
        let mut pop_order = Vec::<Option<u8>>::new();
        while !heap.is_empty() {
            pop_order.push(heap.pop().symbol);
        }
        println!("{:?}", pop_order);
        assert_eq!(pop_order, EXPECTED_POP_ORDER);
    }
}
