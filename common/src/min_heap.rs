use crate::packet::MAX_SYMBOLS;
// use heapless::Vec;

// pub struct MinHeap<T: MinHeapNode + std::cmp::PartialOrd>(Vec<T, MAX_SYMBOLS>);
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
        // MinHeap(Vec::<T, MAX_SYMBOLS>::new())
        MinHeap(Vec::<T>::with_capacity(MAX_SYMBOLS))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.len() == 0
    }

    pub fn push(&mut self, node: T) {
        let _ = self.0.push(node);
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

        while child < end {
            let right = child + 1;
            if right < end && unsafe { self.0.get_unchecked(child) > self.0.get_unchecked(right) } {
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

use heapless::Vec as HeaplessVec;
pub struct MinHeapless<T: MinHeapNode + std::cmp::PartialOrd>(HeaplessVec<T, MAX_SYMBOLS>);

impl<T: MinHeapNode + std::cmp::PartialOrd> Default for MinHeapless<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: MinHeapNode + std::cmp::PartialOrd> MinHeapless<T> {
    pub fn new() -> Self {
        MinHeapless(HeaplessVec::<T, MAX_SYMBOLS>::new())
        // MinHeap(Vec::<T>::with_capacity(MAX_SYMBOLS))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.len() == 0
    }

    pub fn push(&mut self, node: T) {
        let _ = unsafe { self.0.push_unchecked(node) };
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

        while child < end {
            let right = child + 1;
            if right < end && unsafe { self.0.get_unchecked(child) > self.0.get_unchecked(right) } {
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
        let mut pop_order = std::vec::Vec::<Option<u8>>::new();
        while !heap.is_empty() {
            pop_order.push(heap.pop().symbol);
        }
        println!("{:?}", pop_order);
        assert_eq!(pop_order, EXPECTED_POP_ORDER);
    }

    #[test]
    fn pop_order_heapless() {
        let mut heap = MinHeapless::<HeapNode>::new();

        // Tests the integrity of the MinHeap min element ordering.
        for (symbol, frequency) in EXPECTED_SYMBOL_FREQUENCIES {
            heap.push(HeapNode::new(Some(symbol), frequency));
        }

        let mut pop_order = std::vec::Vec::<Option<u8>>::new();
        while !heap.is_empty() {
            let result = heap.pop();
            pop_order.push(result.symbol);
        }
        println!("{:?}", pop_order);
        assert_eq!(pop_order, EXPECTED_POP_ORDER);
    }

    #[test]
    fn min() {
        let mut heap = MinHeap::<HeapNode>::new();
        heap.push(HeapNode::new(Some(1), 1));
        heap.push(HeapNode::new(Some(2), 2));
        heap.push(HeapNode::new(Some(3), 3));
        heap.push(HeapNode::new(Some(17), 17));
        heap.push(HeapNode::new(Some(19), 19));
        heap.push(HeapNode::new(Some(36), 36));
        heap.push(HeapNode::new(Some(7), 7));
        heap.push(HeapNode::new(Some(25), 25));
        heap.push(HeapNode::new(Some(100), 100));

        assert_eq!(heap.pop().frequency, 1);
        assert_eq!(heap.pop().frequency, 2);
        assert_eq!(heap.pop().frequency, 3);
        assert_eq!(heap.pop().frequency, 7);
        assert_eq!(heap.pop().frequency, 17);
        assert_eq!(heap.pop().frequency, 19);
        assert_eq!(heap.pop().frequency, 25);
        assert_eq!(heap.pop().frequency, 36);
        assert_eq!(heap.pop().frequency, 100);
    }
}

// MARK: Benches
#[divan::bench_group(sample_count = 10_000)]
mod benches {
    use super::*;
    use crate::test_cases::*;

    // use divan::{black_box, Bencher};

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

    #[divan::bench]
    fn pop_order() {
        let mut heap = MinHeap::<HeapNode>::new();
        for (symbol, frequency) in EXPECTED_SYMBOL_FREQUENCIES {
            heap.push(HeapNode::new(Some(symbol), frequency));
        }
        let mut pop_order = std::vec::Vec::<Option<u8>>::new();
        while !heap.is_empty() {
            pop_order.push(heap.pop().symbol);
        }
        assert_eq!(pop_order, EXPECTED_POP_ORDER);
    }
}
