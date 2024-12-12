use nohash_hasher::NoHashHasher;
use std::{collections::HashMap, hash::BuildHasherDefault};

use crate::{min_heap::MinHeap, node::TreeNode};

pub(crate) type PrefixMap = HashMap<u8, String, BuildHasherDefault<NoHashHasher<u8>>>;

pub(crate) const MAX_SYMBOLS: usize = 12; // digits 0-9, '|' and '-'

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
}

// Prefix Generation
impl<'a> Packet<'a> {
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
