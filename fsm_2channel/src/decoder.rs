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
    let mut decoded0 = vec![0; packet.decoded_bytes_len as usize + 8];
    let mut decoded1 = vec![0; packet.decoded_bytes_len as usize + 8];

    let mut index0 = 0usize;
    let mut index1 = 0usize;
    let mut state0 = 0usize;
    let mut state1 = 0usize;

    let (last_byte, encoded_bytes) = if packet.encoded_bytes_len as usize % 2 == 1 {
        let (last, rest) = packet.encoded_message.split_last().unwrap();
        (Some(last), rest)
    } else {
        (None, packet.encoded_message)
    };

    let mid = encoded_bytes.len() / 2;
    let (bytes0, bytes1) = encoded_bytes.split_at(mid);
    let mut bit_reader0 = BigEndianReader::new(bytes0);
    let mut bit_reader1 = BigEndianReader::new(bytes1);

    // Lookahead is 56bits
    while bit_reader0.unbuffered_bytes_remaining() > 7 {
        bit_reader0.refill_lookahead();
        bit_reader1.refill_lookahead();
        for _ in 0..7 {
            state0 = step(&mut bit_reader0, table, &mut index0, &mut decoded0, state0);
            state1 = step(&mut bit_reader1, table, &mut index1, &mut decoded1, state1);
        }
    }
    bit_reader0.refill_lookahead();
    bit_reader1.refill_lookahead();
    while bit_reader0.bytes_remaining() > 0 {
        state0 = step(&mut bit_reader0, table, &mut index0, &mut decoded0, state0);
        state1 = step(&mut bit_reader1, table, &mut index1, &mut decoded1, state1);
    }

    state0 = converge(
        bytes1,
        state0,
        state1,
        &mut index0,
        index1,
        &mut decoded0,
        &decoded1,
        table,
    );

    if let Some(last_byte) = last_byte {
        let symbols: &[u8; 9] = &table.tables[state0].symbols[*last_byte as usize];
        copy_symbols(symbols, &mut index0, &mut decoded0);
    }

    // Truncate decoded slop.
    let slice = &decoded0[..packet.decoded_bytes_len as usize];
    std::str::from_utf8(slice).unwrap().to_owned()
}

fn step_state(
    bit_reader: &mut BigEndianReader,
    table: &StateTables,
    index: &mut usize,
    state: usize,
) -> usize {
    let byte = bit_reader.peek(8) as usize;
    let symbols: &[u8; 9] = &table.tables[state].symbols[byte];
    let state = symbols[0] as usize;
    bit_reader.consume(8);
    let symbol_block = u64::from_le_bytes(symbols[1..9].try_into().unwrap());
    let len = 8 - (symbol_block.leading_zeros() / 8) as usize;
    *index += len;
    state
}

fn step(
    bit_reader: &mut BigEndianReader,
    table: &StateTables,
    write_index: &mut usize,
    decoded: &mut [u8],
    state: usize,
) -> usize {
    let index = bit_reader.peek(8) as usize;
    let symbols: &[u8; 9] = &table.tables[state].symbols[index];
    let state = symbols[0] as usize;
    copy_symbols(symbols, write_index, decoded);
    bit_reader.consume(8);
    state
}

#[inline(always)]
fn copy_symbols(symbols: &[u8; 9], write_index: &mut usize, decoded: &mut [u8]) {
    decoded[*write_index..*write_index + 8].copy_from_slice(&symbols[1..9]);
    let symbol_block = u64::from_le_bytes(symbols[1..9].try_into().unwrap());
    let len = 8 - (symbol_block.leading_zeros() / 8) as usize;
    *write_index += len;
}

#[inline(always)]
fn converge(
    bytes1: &[u8],
    mut state0: usize,
    mut state1: usize,
    index0: &mut usize,
    mut index1: usize,
    decoded0: &mut [u8],
    decoded1: &[u8],
    table: &StateTables,
) -> usize {
    let mut bit_reader0 = BigEndianReader::new(bytes1);
    let mut bit_reader1 = BigEndianReader::new(bytes1);

    let prev_state1 = state1;
    let prev_state1_index = index1;
    state1 = 0;
    index1 = 0;

    while bit_reader0.unbuffered_bytes_remaining() > 0 && state0 != state1 {
        bit_reader0.refill_lookahead();
        bit_reader1.refill_lookahead();
        state0 = step(&mut bit_reader0, table, index0, decoded0, state0);
        state1 = step_state(&mut bit_reader1, table, &mut index1, state1);
    }
    while bit_reader0.bytes_remaining() > 0 && state0 != state1 {
        state0 = step(&mut bit_reader0, table, index0, decoded0, state0);
        state1 = step_state(&mut bit_reader1, table, &mut index1, state1);
    }
    if state0 != state1 {
        return state0;
    }

    let copy_len = prev_state1_index - index1;
    decoded0[*index0..*index0 + copy_len].copy_from_slice(&decoded1[index1..index1 + copy_len]);
    *index0 += copy_len;

    prev_state1
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

    let mut heap = symbols_heap(packet);
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

fn symbols_heap(packet: &Packet) -> MinHeapless<HeapNode> {
    let mut heap = MinHeapless::<HeapNode>::new();
    let bytes = &packet.symbol_frequency_bytes;
    for chunk in bytes.chunks_exact(8) {
        let frequency = u32::from_le_bytes(chunk[..4].try_into().unwrap());
        let symbol = chunk[4];
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

fn state_tables(tree: &[TreeNode; MAX_TREE_LEN]) -> StateTables {
    let (table_indices, child_states) = child_states(tree);

    let mut state_tables = StateTables {
        tables: [SymbolTable::default(); MAX_SYMBOLS],
    };

    // Find the last 'state 3' node and populate a symbol table to copy entries from later.
    let reference_index = child_states.iter().rposition(|&x| x == 3).unwrap();
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

fn copy_full_range(
    start_node: &TreeNode,
    tree: &[TreeNode; MAX_TREE_LEN],
    _table_indices: &[u8; MAX_TREE_LEN],
    reference_table: &SymbolTable,
) -> SymbolTable {
    let mut table = SymbolTable {
        symbols: reference_table.symbols,
    };
    table.symbols[0..=127]
        .iter_mut()
        .for_each(|x| x[1] = tree[start_node.left_index as usize].symbol.unwrap());
    table.symbols[128..=255]
        .iter_mut()
        .for_each(|x| x[1] = tree[start_node.right_index as usize].symbol.unwrap());
    table
}

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
            0 => &tree[node.left_index as usize],
            _ => &tree[node.right_index as usize],
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
        table_indices[node.index.unwrap()]
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
    use common::test_cases::*;

    #[test]
    fn decodes_packet() {
        let decoded_message = super::decode_packet(&TEST_BYTES);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
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
        let content = case.request();
        let packet = &Packet::new(&content);
        bencher.bench_local(move || {
            let mut tree = [TreeNode::default(); MAX_TREE_LEN];
            huffman_tree(packet, &mut tree);
            black_box(tree);
        });
    }

    #[divan::bench(sample_count = 100_000, args = [ALL_CASES[0], ALL_CASES[5]])]
    fn gen_table(bencher: Bencher, case: &Case) {
        let content = case.request();
        let packet = &Packet::new(&content);
        bencher.bench_local(move || {
            let mut tree = [TreeNode::default(); MAX_TREE_LEN];
            huffman_tree(packet, &mut tree);
            let table = state_tables(&tree);
            black_box(table);
        });
    }

    #[divan::bench(args = ALL_CASES)]
    fn decode_message(bencher: Bencher, case: &Case) {
        let content = case.request();
        let packet = &Packet::new(&content);
        let mut tree = [TreeNode::default(); MAX_TREE_LEN];
        huffman_tree(packet, &mut tree);
        let table = state_tables(&tree);
        bencher
            .counter(BytesCount::from(packet.decoded_bytes_len))
            .bench_local(move || {
                super::decode_message(black_box(packet), &table);
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
                black_box(super::decode_message(black_box(packet), &table));
            });
    }
}
