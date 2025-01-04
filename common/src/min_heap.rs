use crate::packet::MAX_SYMBOLS;

pub struct MinHeap<T: MinHeapNode + std::cmp::PartialOrd>(Vec<T>);

pub trait MinHeapNode {
    fn frequency(&self) -> u32;
}

impl<T: MinHeapNode + std::cmp::PartialOrd> Default for MinHeap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: MinHeapNode + std::cmp::PartialOrd> MinHeap<T> {
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
        let mut child = self.0.len() - 1;

        while child > 0 {
            let parent = (child - 1) / 2;
            // SAFETY: child is greater than 0 and parent can at minimum be zero.
            if unsafe { self.0.get_unchecked(child) < self.0.get_unchecked(parent) } {
                self.0.swap(child, parent);
                child = parent;
            } else {
                break;
            }
        }
    }

    pub fn pop(&mut self) -> T {
        let root = self.0.swap_remove(0);
        let mut parent = 0;
        let mut child = 1;
        let end = self.0.len();

        while end > child {
            // SAFETY: child is an increasing index and both children are less than end.
            if end > child + 1
                && unsafe { self.0.get_unchecked(child) > self.0.get_unchecked(child + 1) }
            {
                child += 1;
            };

            if unsafe { self.0.get_unchecked(child) < self.0.get_unchecked(parent) } {
                self.0.swap(parent, child);
                parent = child;
                child = 2 * parent + 1;
            } else {
                break;
            }
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
        for (symbol, frequency) in EXPECTED_SYMBOL_FREQUENCIES {
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
