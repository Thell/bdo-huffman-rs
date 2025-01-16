use bitter::{BigEndianReader, BitReader};
use common::min_heap::*;
use common::packet::{Packet, MAX_SYMBOLS};

const MAX_TREE_LEN: usize = 23;

pub fn decode_packet(content: &[u8]) -> String {
    let packet = &Packet::new(content);
    let mut tree = [TreeNode::default(); MAX_TREE_LEN];
    huffman_tree(packet, &mut tree);
    let table = &state_tables(&tree);
    decode_message(packet, table)
}

fn decode_message(packet: &Packet, table: &StateTables) -> String {
    // Add slop space instead of checking write_index against decoded_len.
    let mut decoded: Vec<u8> = Vec::with_capacity(packet.decoded_bytes_len as usize + 8);
    let mut index = 0usize;
    let mut state = 0;

    let mut bit_reader = BigEndianReader::new(packet.encoded_message);

    // Lookahead is 56bits
    // Consume unbuffered bytes; guaranteed 7 8-bit indices per iteration.
    while bit_reader.unbuffered_bytes_remaining() > 7 {
        unsafe {
            bit_reader.refill_lookahead_unchecked();
            state = step(&mut bit_reader, table, &mut index, &mut decoded, state);
            state = step(&mut bit_reader, table, &mut index, &mut decoded, state);
            state = step(&mut bit_reader, table, &mut index, &mut decoded, state);
            state = step(&mut bit_reader, table, &mut index, &mut decoded, state);
            state = step(&mut bit_reader, table, &mut index, &mut decoded, state);
            state = step(&mut bit_reader, table, &mut index, &mut decoded, state);
            state = step(&mut bit_reader, table, &mut index, &mut decoded, state);
        }
    }

    // Drain unbuffered bytes with safe refill.
    while bit_reader.unbuffered_bytes_remaining() > 0 {
        bit_reader.refill_lookahead();
        state = unsafe { step(&mut bit_reader, table, &mut index, &mut decoded, state) };
    }

    // Consume remaining bytes.
    while bit_reader.bytes_remaining() > 0 {
        state = unsafe { step(&mut bit_reader, table, &mut index, &mut decoded, state) };
    }

    // Truncate decoded slop.
    unsafe {
        decoded.set_len(index);
        let slice = std::slice::from_raw_parts(decoded.as_ptr(), decoded.len());
        let mut decoded = std::str::from_utf8_unchecked(slice).to_owned();
        decoded.truncate(packet.decoded_bytes_len as usize);
        decoded
    }
}

#[inline(always)]
unsafe fn step(
    bit_reader: &mut BigEndianReader,
    table: &StateTables,
    write_index: &mut usize,
    decoded: &mut [u8],
    state: usize,
) -> usize {
    let index = bit_reader.peek(8) as usize;
    let symbols: &[u8; 9] = table
        .tables
        .get_unchecked(state)
        .symbols
        .get_unchecked(index);
    let state = symbols[0] as usize;
    copy_symbols(symbols, write_index, decoded);
    bit_reader.consume(8);
    state
}

#[inline(always)]
unsafe fn copy_symbols(symbols: &[u8; 9], write_index: &mut usize, decoded: &mut [u8]) {
    decoded
        .as_mut_ptr()
        .add(*write_index)
        .copy_from_nonoverlapping(symbols.as_ptr().add(1), 8);
    *write_index += symbols[1..].iter().position(|&byte| byte == 0).unwrap_or(8);
}

#[inline(always)]
fn process_heap_node(node: &HeapNode, tree: &mut [TreeNode; MAX_TREE_LEN], index: usize) {
    if node.symbol.is_some() {
        tree[index].symbol = node.symbol;
        tree[index].left_index = 1;
        tree[index].right_index = 2;
    } else {
        tree[index].left_index = tree[node.tree_index as usize].index.unwrap() as u8;
        tree[index].right_index = tree[node.tree_index as usize + 1].index.unwrap() as u8;
    }
    tree[index].index = Some(index);
}

fn huffman_tree(packet: &Packet, tree: &mut [TreeNode; MAX_TREE_LEN]) {
    // Set the root node.
    tree[0].symbol = None;
    tree[0].left_index = 1;
    tree[0].right_index = 2;
    tree[0].index = Some(0);

    let mut heap = unsafe { symbols_heap(packet) };
    let mut tree_index = 2 * packet.symbol_count as usize - 1;

    // Successively move two smallest nodes from heap to tree
    while tree_index > 3 {
        let (left, right) = (heap.pop(), heap.pop());

        // Add heap popped nodes to the tree by setting the existing node values
        tree_index -= 1;
        process_heap_node(&right, tree, tree_index);
        tree_index -= 1;
        process_heap_node(&left, tree, tree_index);

        // Add a parent node to the heap for ordering
        let parent_frequency = left.frequency + right.frequency;
        let parent = HeapNode::new_parent(parent_frequency, tree_index as u8);
        heap.push(parent);
    }

    // Move the last two nodes.
    let (left, right) = (heap.pop(), heap.pop());
    tree_index -= 1;
    process_heap_node(&right, tree, tree_index);
    tree_index -= 1;
    process_heap_node(&left, tree, tree_index);
}

// #[inline(always)]
unsafe fn symbols_heap(packet: &Packet) -> MinHeapless<HeapNode> {
    let mut heap = MinHeapless::<HeapNode>::new();
    let ptr = packet.symbol_frequency_bytes.as_ptr();
    for i in 0..packet.symbol_count as usize {
        let freq_ptr = ptr.add(i * 8) as *const (u32, u8);
        let (frequency, symbol) = freq_ptr.read_unaligned();
        heap.push(HeapNode::new(Some(symbol), frequency));
    }
    heap
}

#[derive(Clone, Copy)]
#[repr(align(4096))]
struct SymbolTable {
    symbols: [[u8; 9]; 256],
}
impl Default for SymbolTable {
    fn default() -> Self {
        SymbolTable {
            symbols: [[0u8; 9]; 256],
        }
    }
}

struct StateTables {
    tables: [SymbolTable; MAX_SYMBOLS],
}

// #[inline(always)]
fn state_tables(tree: &[TreeNode; MAX_TREE_LEN]) -> StateTables {
    let (table_indices, child_states) = child_states(tree);

    let mut state_tables = StateTables {
        tables: [SymbolTable::default(); MAX_SYMBOLS],
    };

    // Find the last 'state 3' node and populate a symbol table to copy entries from later.
    let reference_index = child_states.iter().rposition(|&x| x == 3).unwrap() as usize;
    let reference = initialize_reference_table(&tree[reference_index], tree, &table_indices);
    state_tables.tables[table_indices[reference_index] as usize] = reference;

    for i in 0..MAX_TREE_LEN {
        let table_index = table_indices[i];
        if table_index == MAX_TREE_LEN as u8 || i == reference_index {
            continue;
        }

        // Copy and modify entries from the reference table when we can.
        let start_node = &tree[i];
        let table_fn = match child_states[i] {
            1 => copy_lower_gen_upper,
            2 => gen_lower_copy_upper,
            3 => copy_full_range,
            _ => gen_full_range,
        };
        state_tables.tables[table_index as usize] =
            table_fn(start_node, tree, &table_indices, &reference);
    }
    state_tables
}

// #[inline(always)]
fn child_states(tree: &[TreeNode; MAX_TREE_LEN]) -> ([u8; MAX_TREE_LEN], [u8; MAX_TREE_LEN]) {
    let mut table_indices = [MAX_TREE_LEN as u8; MAX_TREE_LEN];
    let mut child_states = [MAX_TREE_LEN as u8; MAX_TREE_LEN];
    let mut internal_count = 0;
    tree.iter().enumerate().for_each(|(i, node)| {
        if node.symbol.is_none() && node.index.is_some() {
            table_indices[i] = internal_count;
            internal_count += 1;
        };
        let left_state = tree[node.left_index as usize].symbol.is_some() as u8;
        let right_state = tree[node.right_index as usize].symbol.is_some() as u8;
        let child_state = left_state + 2 * right_state;
        child_states[i] = child_state;
    });
    (table_indices, child_states)
}

fn initialize_reference_table(
    start_node: &TreeNode,
    tree: &[TreeNode; MAX_TREE_LEN],
    table_indices: &[u8; MAX_TREE_LEN],
) -> SymbolTable {
    let mut table = SymbolTable::default();
    for byte in 0u8..=127 {
        decode_bits(
            byte,
            start_node,
            &mut table.symbols[byte as usize],
            tree,
            table_indices,
        );
    }
    let (first_half, second_half) = table.symbols.split_at_mut(128);
    second_half.copy_from_slice(first_half);
    table.symbols[128..=255]
        .iter_mut()
        .for_each(|x| x[1] = tree[start_node.right_index as usize].symbol.unwrap());
    table
}

// #[inline(always)]
fn copy_lower_gen_upper(
    start_node: &TreeNode,
    tree: &[TreeNode; MAX_TREE_LEN],
    table_indices: &[u8; MAX_TREE_LEN],
    reference_table: &SymbolTable,
) -> SymbolTable {
    let mut table = SymbolTable::default();
    table.symbols[0..=127].copy_from_slice(&reference_table.symbols[0..=127]);
    table.symbols[0..=127]
        .iter_mut()
        .for_each(|x| x[1] = tree[start_node.left_index as usize].symbol.unwrap());

    for byte in 128u8..=255 {
        decode_bits(
            byte,
            start_node,
            &mut table.symbols[byte as usize],
            tree,
            table_indices,
        );
    }
    table
}

// #[inline(always)]
fn gen_lower_copy_upper(
    start_node: &TreeNode,
    tree: &[TreeNode; MAX_TREE_LEN],
    table_indices: &[u8; MAX_TREE_LEN],
    reference_table: &SymbolTable,
) -> SymbolTable {
    let mut table = SymbolTable::default();
    table.symbols[128..=255].copy_from_slice(&reference_table.symbols[128..=255]);
    table.symbols[128..=255]
        .iter_mut()
        .for_each(|x| x[1] = tree[start_node.right_index as usize].symbol.unwrap());

    for byte in 0u8..=127 {
        decode_bits(
            byte,
            start_node,
            &mut table.symbols[byte as usize],
            tree,
            table_indices,
        );
    }
    table
}

// #[inline(always)]
fn copy_full_range(
    start_node: &TreeNode,
    tree: &[TreeNode; MAX_TREE_LEN],
    _table_indices: &[u8; MAX_TREE_LEN],
    reference_table: &SymbolTable,
) -> SymbolTable {
    let mut table = SymbolTable::default();
    table.symbols = reference_table.symbols;
    table.symbols[0..=127]
        .iter_mut()
        .for_each(|x| x[1] = tree[start_node.left_index as usize].symbol.unwrap());
    table.symbols[128..=255]
        .iter_mut()
        .for_each(|x| x[1] = tree[start_node.right_index as usize].symbol.unwrap());
    table
}

// #[inline(always)]
fn gen_full_range(
    start_node: &TreeNode,
    tree: &[TreeNode; MAX_TREE_LEN],
    table_indices: &[u8; MAX_TREE_LEN],
    _reference_table: &SymbolTable,
) -> SymbolTable {
    let mut table = SymbolTable::default();
    for byte in 0u8..=255 {
        decode_bits(
            byte,
            start_node,
            &mut table.symbols[byte as usize],
            tree,
            table_indices,
        );
    }
    table
}

#[inline(always)]
fn decode_bits<'a>(
    mut bits: u8,
    mut node: &'a TreeNode,
    symbols: &mut [u8; 9],
    tree: &'a [TreeNode; MAX_TREE_LEN],
    table_indices: &[u8; MAX_TREE_LEN],
) {
    let mut write_index = 1;
    for _ in 0..=7 {
        node = match bits >> 7 {
            0 => unsafe { tree.get_unchecked(node.left_index as usize) },
            _ => unsafe { tree.get_unchecked(node.right_index as usize) },
        };
        if let Some(symbol) = node.symbol {
            symbols[write_index] = symbol;
            write_index += 1;
        }
        bits <<= 1;
    }
    symbols[0] = if node.symbol.is_some() {
        0
    } else {
        unsafe { *table_indices.get_unchecked(node.index.unwrap() as usize) as u8 }
    };
}

#[derive(PartialEq, Eq)]
struct HeapNode {
    tree_index: u8,
    symbol: Option<u8>,
    frequency: u32,
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
    fn new(symbol: Option<u8>, frequency: u32) -> Self {
        Self {
            tree_index: 0,
            symbol,
            frequency,
        }
    }
    fn new_parent(frequency: u32, left_index: u8) -> Self {
        Self {
            tree_index: left_index,
            symbol: None,
            frequency,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TreeNode {
    left_index: u8,
    right_index: u8,
    symbol: Option<u8>,
    frequency: u32,
    index: Option<usize>,
}

// =========================================================
// MARK: Unit Tests

#[cfg(test)]
mod tests {
    use common::{packet::Packet, test_cases::*};

    #[test]
    fn decodes_packet() {
        let decoded_message = super::decode_packet(&TEST_BYTES);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn gen_table() {
        let response_bytes = TEST_BYTES;
        let packet = &Packet::new(&response_bytes);
        let mut tree = [super::TreeNode::default(); super::MAX_TREE_LEN];
        super::huffman_tree(packet, &mut tree);
        for (i, node) in tree.iter().enumerate() {
            println!("node {}: {:?}", i, node);
        }
        let table = super::state_tables(&tree);
        for t in table.tables {
            for i in 0..=255 {
                println!("{} {:?}", i, t.symbols[i]);
            }
            // for i in 123..133 {
            //     println!("{} {:?}", i, t.symbols[i]);
            // }
            // for i in 246..=255 {
            //     println!("{} {:?}", i, t.symbols[i]);
            // }
        }
    }
}

// MARK: Benches

use common::test_cases::BENCH_SAMPLE_COUNT;
#[divan::bench_group(sample_count = BENCH_SAMPLE_COUNT)]
mod bench {
    use super::*;
    use common::test_cases::*;

    use divan::counter::BytesCount;
    use divan::{black_box, Bencher};

    #[divan::bench(sample_count = 100_000, args = [ALL_CASES[0], ALL_CASES[5]])]
    fn gen_tree(bencher: Bencher, case: &Case) {
        let response_bytes = case.request();
        let packet = &Packet::new(&response_bytes);
        bencher.bench_local(move || {
            let mut tree = [TreeNode::default(); MAX_TREE_LEN];
            huffman_tree(packet, &mut tree);
            black_box(tree);
        });
    }

    #[divan::bench(sample_count = 100_000, args = [ALL_CASES[0], ALL_CASES[5]])]
    fn gen_table(bencher: Bencher, case: &Case) {
        let response_bytes = case.request();
        let packet = &Packet::new(&response_bytes);
        bencher.bench_local(move || {
            let mut tree = [TreeNode::default(); MAX_TREE_LEN];
            huffman_tree(packet, &mut tree);
            let table = state_tables(&tree);
            black_box(table);
        });
    }

    #[divan::bench(args = ALL_CASES)]
    fn decode_message(bencher: Bencher, case: &Case) {
        let response_bytes = case.request();
        let packet = &Packet::new(&response_bytes);
        let mut tree = [TreeNode::default(); MAX_TREE_LEN];
        huffman_tree(packet, &mut tree);
        let table = state_tables(&tree);
        bencher
            .counter(BytesCount::from(packet.decoded_bytes_len))
            .bench_local(move || {
                super::decode_message(black_box(&packet), &table);
            });
    }

    #[divan::bench(args = ALL_CASES)]
    fn decode_packet(bencher: Bencher, case: &Case) {
        let content = case.request();
        bencher.bench_local(move || {
            super::decode_packet(black_box(&content));
        });
    }

    #[divan::bench(args = [ALL_CASES[0]])]
    fn decode_message2x(bencher: Bencher, case: &Case) {
        let content = case.request();
        let content2 = content.clone();
        let packet = &Packet::new(&content);
        let mut tree = [TreeNode::default(); MAX_TREE_LEN];
        huffman_tree(packet, &mut tree);
        let table = state_tables(&tree);
        bencher
            .counter(BytesCount::from(2 * packet.decoded_bytes_len))
            .bench_local(move || {
                black_box(super::decode_packet(black_box(&content2)));
                black_box(super::decode_message(black_box(&packet), &table));
            });
    }
}
