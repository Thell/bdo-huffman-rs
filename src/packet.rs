use nohash_hasher::NoHashHasher;
use std::{collections::HashMap, hash::BuildHasherDefault};

use crate::{
    min_heap::{MinHeap, MinHeapNode},
    node::{FlatNode, TreeNode},
};

pub(crate) type PrefixMap = HashMap<u8, String, BuildHasherDefault<NoHashHasher<u8>>>;

pub(crate) const MAX_SYMBOLS: usize = 12; // digits 0-9, '|' and '-'

pub struct Packet<'a> {
    pub len: u64,
    pub symbol_count: u32,
    pub symbol_table_bytes: &'a [u8],
    pub bitstream_len: u32,
    pub encoded_bytes_len: u32,
    pub decoded_bytes_len: u32,
    pub encoded_message: &'a [u8],
}

impl<'a> Packet<'a> {
    pub fn new(content: &'a [u8]) -> Self {
        let mut pos = 0;

        let u64_bytes: [u8; 8] = content[pos..pos + 8].try_into().unwrap();
        let len = u64::from_le_bytes(u64_bytes);
        pos += 8;

        let mut u32_bytes: [u8; 4] = content[pos..pos + 4].try_into().unwrap();
        let symbol_count = u32::from_le_bytes(u32_bytes);
        pos += 4;

        let symbol_table_bytes = &content[pos..pos + 8 * symbol_count as usize];
        pos += 8 * symbol_count as usize;

        u32_bytes = content[pos..pos + 4].try_into().unwrap();
        let bitstream_len = u32::from_le_bytes(u32_bytes);
        pos += 4;

        u32_bytes = content[pos..pos + 4].try_into().unwrap();
        let encoded_bytes_len = u32::from_le_bytes(u32_bytes);
        pos += 4;

        u32_bytes = content[pos..pos + 4].try_into().unwrap();
        let decoded_bytes_len = u32::from_le_bytes(u32_bytes);
        pos += 4;

        let encoded_message = &content[pos..pos + encoded_bytes_len as usize];

        Packet {
            len,
            symbol_count,
            symbol_table_bytes,
            bitstream_len,
            encoded_bytes_len,
            decoded_bytes_len,
            encoded_message,
        }
    }

    pub fn symbols_table(&self) -> Vec<(u8, u32)> {
        let mut frequencies = [(0u8, 0u32); 12];
        let ptr = self.symbol_table_bytes.as_ptr();
        unsafe {
            for i in 0..self.symbol_count {
                let freq_ptr = ptr.add(i as usize * 8) as *const u32;
                let symbol_ptr = ptr.add(i as usize * 8 + 4);

                let frequency = freq_ptr.read_unaligned();
                let symbol = symbol_ptr.read();

                *frequencies.get_unchecked_mut(i as usize) = (symbol, frequency);
            }
        }

        frequencies[..self.symbol_count as usize].to_vec()
    }
}

// Huffman Tree Generation
impl Packet<'_> {
    #[inline(always)]
    pub fn symbols_heap<T: MinHeapNode>(&self) -> MinHeap<T> {
        let mut heap = MinHeap::<T>::with_capacity(MAX_SYMBOLS);
        let ptr = self.symbol_table_bytes.as_ptr();
        unsafe {
            for i in 0..self.symbol_count {
                let freq_ptr = ptr.add(i as usize * 8) as *const u32;
                let symbol_ptr = ptr.add(i as usize * 8 + 4);

                let frequency = freq_ptr.read_unaligned();
                let symbol = symbol_ptr.read();
                heap.push(T::new(Some(symbol), frequency));
            }
        }
        heap
    }

    pub fn treenode_tree(&self) -> TreeNode {
        let mut heap = self.symbols_heap::<TreeNode>();

        let mut size = heap.len();
        while size > 1 {
            // Move two smallest nodes from heap to vec ensuring smallest on the left
            let left = heap.pop();
            let right = heap.pop();

            // Add parent node to the heap for ordering
            heap.push(TreeNode::new_parent(left, right));
            size -= 1;
        }

        // Move the last node (the root)
        heap.pop()
    }

    pub fn flatnode_tree(&self) -> Vec<FlatNode> {
        let mut heap = self.symbols_heap::<FlatNode>();

        let mut right_index = 2 * self.symbol_count as usize - 1;
        let mut nodes = vec![FlatNode::default(); right_index];
        let nodes_ptr = nodes.as_mut_ptr();
        right_index -= 1;

        loop {
            // Move two smallest nodes from heap to vec ensuring smallest on the left
            let smallest = heap.pop();
            let next_smallest = heap.pop();
            let parent_freq = smallest.frequency + next_smallest.frequency;

            unsafe {
                *nodes_ptr.add(right_index - 1) = smallest;
                *nodes_ptr.add(right_index) = next_smallest;
            }

            // Add parent node to the heap for ordering
            heap.push(FlatNode::new_parent(
                parent_freq,
                unsafe { nodes_ptr.add(right_index - 1) },
                unsafe { nodes_ptr.add(right_index) },
            ));

            right_index -= 2;
            if right_index < 2 {
                // Move the last node (the root)
                unsafe {
                    *nodes_ptr = heap.pop();
                }
                break;
            }
        }

        nodes
    }
}

// Prefix Generation
impl Packet<'_> {
    // This only exists for testing proper tree building and benchmarking traversal.
    pub fn treenode_prefix_map(&self, tree: &TreeNode) -> PrefixMap {
        fn traverse(node: &TreeNode, prefix: String, prefixes: &mut PrefixMap) {
            if let Some(symbol) = node.symbol {
                prefixes.insert(symbol, prefix);
            } else {
                if let Some(left) = &node.left_child {
                    traverse(left, format!("{}0", prefix), prefixes);
                }
                if let Some(right) = &node.right_child {
                    traverse(right, format!("{}1", prefix), prefixes);
                }
            }
        }

        let mut prefixes: PrefixMap =
            HashMap::with_capacity_and_hasher(12, BuildHasherDefault::default());
        traverse(tree, String::new(), &mut prefixes);
        prefixes
    }

    // This only exists for testing proper tree building and benchmarking traversal.
    pub fn flatnode_prefix_map(&self, tree: &[FlatNode]) -> PrefixMap {
        fn traverse(node: &FlatNode, prefix: String, prefixes: &mut PrefixMap) {
            if let Some(symbol) = node.symbol {
                prefixes.insert(symbol, prefix);
            } else {
                unsafe {
                    traverse(&*node.left_ptr, format!("{}0", prefix), prefixes);
                    traverse(&*node.right_ptr, format!("{}1", prefix), prefixes);
                }
            }
        }

        let mut prefixes: PrefixMap =
            HashMap::with_capacity_and_hasher(12, BuildHasherDefault::default());
        traverse(&tree[0], String::new(), &mut prefixes);
        prefixes
    }
}

// =========================================================

// MARK: Unit Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_cases::*;

    #[test]
    fn parses_symbol_table() {
        // Tests the integrity of the packet symbol parsing.
        let packet = &Packet::new(&TEST_BYTES);
        assert!(packet.symbols_table() == EXPECTED_SYMBOL_TABLE);
    }

    #[test]
    fn treenode_tree() {
        // Tests the integrity of the MinHeap nested tree building.
        let packet = &Packet::new(&TEST_BYTES);
        let nested_tree = packet.treenode_tree();

        let built_prefixes = packet.treenode_prefix_map(&nested_tree);

        for (symbol, test_prefix) in EXPECTED_PREFIXES.into_iter() {
            let built_prefix = built_prefixes.get(&symbol.as_bytes()[0]).unwrap();
            println!(
                "symbol: {} [{}]: prefix: {} => expected: {}",
                symbol,
                symbol.as_bytes()[0],
                test_prefix,
                built_prefix
            );
            assert_eq!(built_prefix, test_prefix, "Prefix test {} failed!", symbol);
        }
    }

    #[test]
    fn flatnode_tree() {
        // Tests the integrity of the MinHeap tree building.
        let packet = &Packet::new(&TEST_BYTES);
        let root = packet.flatnode_tree();

        let built_prefixes = packet.flatnode_prefix_map(&root);

        for (symbol, test_prefix) in EXPECTED_PREFIXES.into_iter() {
            let built_prefix = built_prefixes.get(&symbol.as_bytes()[0]).unwrap();
            println!(
                "symbol: {} [{}]: prefix: {} => expected: {}",
                symbol,
                symbol.as_bytes()[0],
                test_prefix,
                built_prefix
            );
            assert_eq!(built_prefix, test_prefix, "Prefix test {} failed!", symbol);
        }
    }

    #[test]
    fn trees_cmp_all_samples() {
        // Tests the equality of all trees generated.
        for case in SAMPLE_CASES {
            println!("case: {}", case.name);
            let content = case.request();
            let packet = &Packet::new(&content);

            let nested_tree = packet.treenode_tree();
            let nested_tree_prefixes = packet.treenode_prefix_map(&nested_tree);

            let flat_tree = packet.flatnode_tree();
            let flat_tree_prefixes = packet.flatnode_prefix_map(&flat_tree);

            for key in flat_tree_prefixes.clone().into_keys() {
                assert_eq!(
                    flat_tree_prefixes.get_key_value(&key),
                    nested_tree_prefixes.get_key_value(&key)
                );
            }
        }
    }
}

// MARK: Benches
#[divan::bench_group(sample_count = 10_000)]
mod common {
    use super::*;
    use crate::test_cases::{Case, ALL_CASES};

    use divan::{black_box, Bencher};

    #[divan::bench(args = [ALL_CASES[0], ALL_CASES[5]])]
    fn packet_from_content(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();

        bencher.bench_local(move || {
            black_box(Packet::new(response_bytes));
        });
    }

    #[divan::bench(args = [ALL_CASES[0], ALL_CASES[5]])]
    fn symbols_table(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = Packet::new(response_bytes);

        bencher.bench_local(move || {
            black_box(&packet.symbols_table());
        });
    }

    #[divan::bench(types = [TreeNode, FlatNode], args = [ALL_CASES[0], ALL_CASES[5]])]
    fn symbols_heap<T: MinHeapNode>(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = &Packet::new(response_bytes);
        bencher.bench_local(move || {
            black_box(packet.symbols_heap::<T>());
        });
    }

    #[divan::bench(types = [TreeNode, FlatNode], args = [ALL_CASES[0], ALL_CASES[5]])]
    fn tree<T: MinHeapNode + 'static>(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = &Packet::new(response_bytes);

        bencher.bench_local(move || {
            if std::any::TypeId::of::<T>() == std::any::TypeId::of::<TreeNode>() {
                black_box(packet.treenode_tree());
            } else if std::any::TypeId::of::<T>() == std::any::TypeId::of::<FlatNode>() {
                black_box(packet.flatnode_tree());
            };
        });
    }

    #[divan::bench(types = [TreeNode, FlatNode], args = [ALL_CASES[0], ALL_CASES[5]])]
    fn prefix_map<T: MinHeapNode + 'static>(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = &Packet::new(response_bytes);

        let treenode_tree = packet.treenode_tree();
        let flatnode_tree = packet.flatnode_tree();

        bencher.bench_local(move || {
            if std::any::TypeId::of::<T>() == std::any::TypeId::of::<TreeNode>() {
                packet.treenode_prefix_map(black_box(&treenode_tree));
            } else if std::any::TypeId::of::<T>() == std::any::TypeId::of::<FlatNode>() {
                packet.flatnode_prefix_map(black_box(&flatnode_tree));
            }
        });
    }
}
