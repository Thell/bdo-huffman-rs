use nohash_hasher::NoHashHasher;
use std::{collections::HashMap, hash::BuildHasherDefault};

use crate::{
    min_heap::{MinHeap, MinHeapNode},
    node::{FlatNode, FlatNodeSafe, TreeNode},
};

pub(crate) type PrefixMap = HashMap<u8, String, BuildHasherDefault<NoHashHasher<u8>>>;

pub(crate) const MAX_SYMBOLS: usize = 12; // digits 0-9, '|' and '-'

// Extended prefixes in flat array layout.
#[repr(C)]
pub struct PrefixTable {
    pub bits_used: [u8; 256],
    pub lens: [u8; 256],
    pub symbols: [[u8; 6]; 256],
}

impl Default for PrefixTable {
    fn default() -> Self {
        Self::new()
    }
}

impl PrefixTable {
    pub fn new() -> Self {
        PrefixTable {
            bits_used: [0u8; 256],
            lens: [0u8; 256],
            symbols: [[0u8; 6]; 256],
        }
    }
}

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
        let mut heap = self.symbols_heap::<FlatNodeSafe>();

        let mut right_index = 2 * self.symbol_count as usize - 1;
        let mut tree = vec![FlatNode::default(); right_index];
        right_index -= 1;

        loop {
            // Move two smallest nodes from heap to vec ensuring smallest on the left
            let left = heap.pop();
            let right = heap.pop();
            let parent_frequency = left.frequency + right.frequency;

            // Add popped nodes to the tree vec by setting the existing node values
            tree[right_index - 1].symbol = left.symbol;
            tree[right_index].symbol = right.symbol;
            tree[right_index - 1].left_ptr = &tree[left.left_index as usize] as *const FlatNode;
            tree[right_index].left_ptr = &tree[right.left_index as usize] as *const FlatNode;
            tree[right_index - 1].right_ptr = &tree[left.right_index as usize] as *const FlatNode;
            tree[right_index].right_ptr = &tree[right.right_index as usize] as *const FlatNode;

            // Add a parent node to the heap for ordering
            heap.push(FlatNodeSafe::new_parent(
                parent_frequency,
                right_index as u8 - 1,
                right_index as u8,
            ));

            right_index -= 2;
            if right_index < 2 {
                // Move the last node (the root) to the tree vec
                let root = heap.pop();
                tree[0].symbol = root.symbol;
                tree[0].left_ptr = &tree[root.left_index as usize] as *const FlatNode;
                tree[0].right_ptr = &tree[root.right_index as usize] as *const FlatNode;
                break;
            }
        }
        tree
    }

    pub fn flatnode_tree_safe(&self) -> Vec<FlatNodeSafe> {
        let mut heap = self.symbols_heap::<FlatNodeSafe>();

        let mut right_index = 2 * self.symbol_count as usize - 1;
        let mut tree = vec![FlatNodeSafe::default(); right_index];
        right_index -= 1;

        loop {
            // Move two smallest nodes from heap to vec ensuring smallest on the left
            let left = heap.pop();
            let right = heap.pop();
            let parent_frequency = left.frequency + right.frequency;

            // Add children to the tree vec
            tree[right_index - 1] = left;
            tree[right_index] = right;

            // Add parent node to the heap for ordering
            heap.push(FlatNodeSafe::new_parent(
                parent_frequency,
                right_index as u8 - 1,
                right_index as u8,
            ));

            right_index -= 2;
            if right_index < 2 {
                // Move the last node (the root) to the tree vec
                tree[0] = heap.pop();
                break;
            }
        }
        tree
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

    #[allow(clippy::unnecessary_cast)]
    pub fn flatnode_prefix_table(&self, tree: &[FlatNode]) -> PrefixTable {
        // Generate a multi-symbol lookup table.
        // Decodes all 8 step paths through the tree storing each symbol visited,
        // the number of symbols written, and the number of bits used when the
        // last symbol was visited.
        let mut prefix_entry = PrefixTable::new();
        let root = unsafe { tree.get_unchecked(0) };

        for byte in 0u8..=255 {
            let symbols_ptr = unsafe {
                prefix_entry
                    .symbols
                    .get_unchecked_mut(byte as usize)
                    .as_mut_ptr()
            };

            let mut node = root;
            let mut bits = byte;
            let mut bits_used = 0;
            let mut write_index = 0;

            for i in 0..8 {
                unsafe {
                    let direction = ((bits & 0b1000_0000) != 0) as usize;
                    node = (*(&node.left_ptr as *const _ as *const *const FlatNode)
                        .add(direction)
                        .as_ref()
                        .unwrap_unchecked())
                    .as_ref()
                    .unwrap_unchecked();

                    if let Some(symbol) = node.symbol {
                        *symbols_ptr.add(write_index) = symbol;
                        write_index += 1;
                        bits_used = i + 1;
                        if write_index == 6 {
                            break;
                        }
                        node = root;
                    }
                    bits <<= 1;
                }
            }

            unsafe { *prefix_entry.lens.get_unchecked_mut(byte as usize) = write_index as u8 };
            prefix_entry.bits_used[byte as usize] = bits_used;
        }

        prefix_entry
    }

    pub fn flatnode_prefix_table_safe(&self, tree: &[FlatNodeSafe]) -> PrefixTable {
        // Generate a multi-symbol lookup table.
        // Decodes all 8 step paths through the tree storing each symbol visited,
        // the number of symbols written, and the number of bits used when the
        // last symbol was visited.
        let mut prefix_entry = PrefixTable::new();
        let root = &tree[0];

        for byte in 0u8..=255 {
            let symbols = &mut prefix_entry.symbols[byte as usize];
            let mut node = root;
            let mut bits = byte;
            let mut bits_used = 0;
            let mut write_index = 0;

            for i in 0..8 {
                let direction = (bits & 0b1000_0000) != 0;
                node = match direction {
                    true => &tree[node.right_index as usize],
                    false => &tree[node.left_index as usize],
                };
                if let Some(symbol) = node.symbol {
                    symbols[write_index] = symbol;
                    write_index += 1;
                    bits_used = i + 1;
                    if write_index == 6 {
                        break;
                    }
                    node = root;
                }
                bits <<= 1;
            }

            prefix_entry.lens[byte as usize] = write_index as u8;
            prefix_entry.bits_used[byte as usize] = bits_used;
        }

        prefix_entry
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
        let packet = &mut Packet::new(&TEST_BYTES);
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
            let packet = &mut Packet::new(&content);

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

    #[divan::bench(types = [TreeNode, FlatNodeSafe], args = [ALL_CASES[0], ALL_CASES[5]])]
    fn symbols_heap<T: MinHeapNode>(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = &Packet::new(response_bytes);
        bencher.bench_local(move || {
            black_box(packet.symbols_heap::<T>());
        });
    }

    #[divan::bench(types = [TreeNode, FlatNode, FlatNodeSafe], args = [ALL_CASES[0], ALL_CASES[5]])]
    fn tree<T: 'static>(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = &mut Packet::new(response_bytes);

        bencher.bench_local(move || {
            if std::any::TypeId::of::<T>() == std::any::TypeId::of::<TreeNode>() {
                black_box(packet.treenode_tree());
            } else if std::any::TypeId::of::<T>() == std::any::TypeId::of::<FlatNode>() {
                black_box(packet.flatnode_tree());
            } else if std::any::TypeId::of::<T>() == std::any::TypeId::of::<FlatNodeSafe>() {
                black_box(packet.flatnode_tree_safe());
            }
        });
    }

    #[divan::bench(types = [TreeNode, FlatNode], args = [ALL_CASES[0], ALL_CASES[5]])]
    fn prefix_map<T: 'static>(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = &mut Packet::new(response_bytes);

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

#[divan::bench_group(sample_count = 10_000)]
mod flatnode_multi_symbol {
    use super::*;
    use crate::test_cases::{Case, ALL_CASES};

    use divan::{black_box, Bencher};

    #[allow(clippy::extra_unused_type_parameters)]
    #[divan::bench(types = [FlatNode, FlatNodeSafe], args = [ALL_CASES[0], ALL_CASES[5]])]
    fn prefix_table<T: 'static>(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = &mut Packet::new(response_bytes);
        let flatnode_tree = packet.flatnode_tree();
        let flatnode_tree_safe = packet.flatnode_tree_safe();

        bencher.bench_local(move || {
            if std::any::TypeId::of::<T>() == std::any::TypeId::of::<FlatNode>() {
                packet.flatnode_prefix_table(black_box(&flatnode_tree));
            } else if std::any::TypeId::of::<T>() == std::any::TypeId::of::<FlatNodeSafe>() {
                packet.flatnode_prefix_table_safe(black_box(&flatnode_tree_safe));
            }
        });
    }
}
