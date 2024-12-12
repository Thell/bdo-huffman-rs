use nohash_hasher::NoHashHasher;
use std::{collections::HashMap, hash::BuildHasherDefault};

use crate::{
    min_heap::MinHeap,
    node::{FlatNode, TreeNode},
};

pub(crate) type PrefixMap = HashMap<u8, String, BuildHasherDefault<NoHashHasher<u8>>>;

pub(crate) const MAX_SYMBOLS: usize = 12; // digits 0-9, '|' and '-'

// Extended prefix contains the symbols for 1 encoded byte. The actual prefix is the bit
// representation of a u8 (0..=255) truncated at `used_bits`.
// Since prefix code lengths from the incoming symbol table is from 0 to 7 bits multiple
// symbols can fit in a byte and there aren't any bytes without a symbol which allows a
// table of 256 entries to work nicely as a multi-symbol lookup table.
#[derive(Debug, Clone)]
pub struct ExtendedPrefix {
    pub symbols: Vec<u8>,
    pub used_bits: usize,
}

impl ExtendedPrefix {
    fn new() -> Self {
        ExtendedPrefix {
            symbols: Vec::<u8>::with_capacity(8),
            used_bits: 0,
        }
    }
}

/// A zero-copy representation of a packet, parsing directly from the input
/// data by taking ownership, avoiding allocation and redundant copying.
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
    // Creates a `Packet` by taking ownership of `content`, enabling zero-copy
    // parsing to avoid allocating new storage and redundant copying.
    // This reduces the runtime of large packet parsing from ~440ns to 3.2ns.
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

    pub fn symbol_table(&self) -> Vec<(u8, u32)> {
        let mut pos = 0;
        let mut frequencies = Vec::with_capacity(MAX_SYMBOLS);

        let bytes = &self.symbol_table_bytes;
        for _ in 0..self.symbol_count {
            let freq_bytes: [u8; 4] = bytes[pos..pos + 4].try_into().unwrap();
            let frequency = u32::from_le_bytes(freq_bytes);
            let symbol = bytes[pos + 4];
            frequencies.push((symbol, frequency));
            pos += 8;
        }
        frequencies
    }
}

// Tree Generation
impl<'a> Packet<'a> {
    pub fn nested_tree(&self, symbol_table: &[(u8, u32)]) -> TreeNode {
        let mut heap: MinHeap<TreeNode> = MinHeap::new();
        for (symbol, freq) in symbol_table.iter() {
            heap.push(TreeNode::new(Some(*symbol), *freq));
        }

        let mut size = heap.len();
        while size > 1 {
            let left = heap.pop();
            let right = heap.pop();
            heap.push(TreeNode {
                symbol: None,
                frequency: left.frequency + right.frequency,
                left_child: Some(Box::new(left)),
                right_child: Some(Box::new(right)),
            });
            size -= 1;
        }
        heap.pop()
    }

    pub fn flat_tree(&self, frequencies: &[(u8, u32)]) -> Vec<FlatNode> {
        let mut heap = MinHeap::<FlatNode>::with_capacity(MAX_SYMBOLS);
        for &(symbol, freq) in frequencies {
            heap.push(FlatNode::new(Some(symbol), freq));
        }

        let mut right_index = 2 * frequencies.len() - 1;
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
            heap.push(FlatNode {
                symbol: None,
                frequency: parent_freq,
                left_ptr: unsafe { nodes_ptr.add(right_index - 1) },
                right_ptr: unsafe { nodes_ptr.add(right_index) },
            });

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
impl<'a> Packet<'a> {
    // This only exists for testing proper tree building and benchmarking traversal.
    fn prefixes_from_treenode(&self, nested_tree: &TreeNode) -> PrefixMap {
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
        traverse(nested_tree, String::new(), &mut prefixes);
        prefixes
    }

    // This only exists for testing proper tree building and benchmarking traversal.
    pub fn prefixes_from_flatnode(&self, flat_tree: &[FlatNode]) -> PrefixMap {
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
        traverse(&flat_tree[0], String::new(), &mut prefixes);
        prefixes
    }

    // Fully decodes a single byte storing visited symbols and bits used up to the last symbol.
    fn extended_prefix_from_byte(&self, byte: u8, tree: &[FlatNode], prefix: &mut ExtendedPrefix) {
        let root = unsafe { tree.get_unchecked(0) };
        let mut node = root;
        let mut write_index = 0;
        let mut bits = byte;

        for i in 0..8 {
            node = match bits & 0b1000_0000 {
                0b0 => unsafe { node.left_ptr.as_ref().unwrap_unchecked() },
                _ => unsafe { node.right_ptr.as_ref().unwrap_unchecked() },
            };
            bits <<= 1;

            if let Some(symbol) = node.symbol {
                prefix.symbols.push(symbol);
                write_index += 1;
                prefix.used_bits = i + 1;
                node = root;
            }
        }

        unsafe {
            prefix.symbols.set_len(write_index);
        }
    }

    pub fn extended_prefixes(&self, tree: &[FlatNode]) -> Vec<ExtendedPrefix> {
        let mut table = vec![ExtendedPrefix::new(); 256];
        for byte in 0u8..=255 {
            let element = unsafe { table.get_unchecked_mut(byte as usize) };
            self.extended_prefix_from_byte(byte, tree, element)
        }
        table
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
        assert!(packet.symbol_table() == EXPECTED_SYMBOL_TABLE);
    }

    #[test]
    fn builds_prefix_nested_tree() {
        // Tests the integrity of the MinHeap nested tree building.
        let packet = &Packet::new(&TEST_BYTES);
        let nested_tree = packet.nested_tree(&EXPECTED_SYMBOL_TABLE);

        let built_prefixes = packet.prefixes_from_treenode(&nested_tree);

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
    fn builds_prefix_flat_tree() {
        // Tests the integrity of the MinHeap tree building.
        let packet = &Packet::new(&TEST_BYTES);
        let root = packet.flat_tree(&EXPECTED_SYMBOL_TABLE);

        let built_prefixes = packet.prefixes_from_flatnode(&root);

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
}

// MARK: Benches

#[divan::bench_group(sample_count = 10_000)]
mod benches_common {
    use super::*;
    use crate::test_cases::{Case, ALL_CASES};

    use divan::counter::{BytesCount, ItemsCount};
    use divan::{black_box, Bencher};

    #[divan::bench(args = ALL_CASES)]
    fn packet_parsing(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();

        bencher
            .counter(ItemsCount::from(1usize))
            .bench_local(move || {
                Packet::new(black_box(response_bytes));
            });
    }

    #[divan::bench(args = ALL_CASES)]
    fn symbol_table_building(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = Packet::new(response_bytes);

        bencher
            .counter(BytesCount::from(packet.symbol_table_bytes.len()))
            .bench_local(move || {
                black_box(&packet.symbol_table());
            });
    }
}

#[divan::bench_group(sample_count = 10_000)]
mod benches_treenode {
    use super::*;
    use crate::test_cases::{Case, ALL_CASES};

    use divan::counter::BytesCount;
    use divan::{black_box, Bencher};

    #[divan::bench(args = ALL_CASES)]
    fn tree_building_nested(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = &Packet::new(response_bytes);
        let symbol_table = &packet.symbol_table();
        let num_nodes = 2 * symbol_table.len() - 1;

        bencher
            .counter(BytesCount::from(
                num_nodes * std::mem::size_of::<TreeNode>(),
            ))
            .bench_local(move || {
                packet.nested_tree(black_box(symbol_table));
            });
    }

    #[divan::bench(args = ALL_CASES)]
    fn prefix_building_nested(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = &Packet::new(response_bytes);
        let symbol_table = &packet.symbol_table();
        let tree = packet.nested_tree(symbol_table);
        let num_nodes = 2 * symbol_table.len() - 1;

        bencher
            .counter(BytesCount::from(num_nodes))
            .bench_local(move || {
                packet.prefixes_from_treenode(black_box(&tree));
            });
    }
}

#[divan::bench_group(sample_count = 10_000)]
mod benches_flatnode {
    use super::*;
    use crate::test_cases::{Case, ALL_CASES};

    use divan::counter::BytesCount;
    use divan::{black_box, Bencher};

    #[divan::bench(args = ALL_CASES)]
    fn tree_building_flat(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = &Packet::new(response_bytes);
        let symbol_table = &packet.symbol_table();
        let num_nodes = 2 * symbol_table.len() - 1;

        bencher
            .counter(BytesCount::from(
                num_nodes * std::mem::size_of::<FlatNode>(),
            ))
            .bench_local(move || {
                packet.flat_tree(black_box(symbol_table));
            });
    }

    #[divan::bench(args = ALL_CASES)]
    fn prefix_building_flat(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = &Packet::new(response_bytes);
        let symbol_table = &packet.symbol_table();
        let tree = packet.flat_tree(symbol_table);
        let num_nodes = 2 * symbol_table.len() - 1;

        bencher
            .counter(BytesCount::from(num_nodes))
            .bench_local(move || {
                packet.prefixes_from_flatnode(black_box(&tree));
            });
    }

    #[divan::bench(args = ALL_CASES)]
    fn prefix_building_flat_extended(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = &Packet::new(response_bytes);
        let symbol_table = &packet.symbol_table();
        let tree = packet.flat_tree(symbol_table);
        let num_nodes = 2 * symbol_table.len() - 1;

        bencher
            .counter(BytesCount::from(num_nodes))
            .bench_local(move || {
                packet.extended_prefixes(black_box(&tree));
            });
    }
}
