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
    let mut decoded0: Vec<u8> = Vec::with_capacity(packet.decoded_bytes_len as usize + 8);
    let mut decoded1: Vec<u8> = Vec::with_capacity(packet.decoded_bytes_len as usize + 8);
    let mut decoded2: Vec<u8> = Vec::with_capacity(packet.decoded_bytes_len as usize + 8);
    let mut decoded3: Vec<u8> = Vec::with_capacity(packet.decoded_bytes_len as usize + 8);

    let mut ptr0 = decoded0.as_mut_ptr();
    let mut ptr1 = decoded1.as_mut_ptr();
    let mut ptr2 = decoded2.as_mut_ptr();
    let mut ptr3 = decoded3.as_mut_ptr();

    let mut state0 = 0usize;
    let mut state1 = 0usize;
    let mut state2 = 0usize;
    let mut state3 = 0usize;

    let (tail, encoded_bytes) = if packet.encoded_bytes_len as usize % 4 != 0 {
        let n = (packet.encoded_bytes_len / 4) * 4;
        let (encoded_bytes, tail) = packet.encoded_message.split_at(n as usize);
        (Some(tail), encoded_bytes)
    } else {
        (None, packet.encoded_message)
    };

    let chunk_len = encoded_bytes.len() / 4;
    let mut chunk_iter = encoded_bytes.chunks_exact(chunk_len);
    let bytes0 = chunk_iter.next().unwrap();
    let bytes1 = chunk_iter.next().unwrap();
    let bytes2 = chunk_iter.next().unwrap();
    let bytes3 = chunk_iter.next().unwrap();

    let mut bit_reader0 = BigEndianReader::new(bytes0);
    let mut bit_reader1 = BigEndianReader::new(bytes1);
    let mut bit_reader2 = BigEndianReader::new(bytes2);
    let mut bit_reader3 = BigEndianReader::new(bytes3);

    unsafe {
        // Lookahead is 56 bits
        while bit_reader0.unbuffered_bytes_remaining() > 7 {
            bit_reader0.refill_lookahead_unchecked();
            bit_reader1.refill_lookahead_unchecked();
            bit_reader2.refill_lookahead_unchecked();
            bit_reader3.refill_lookahead_unchecked();
            for _ in 0..7 {
                state0 = step(&mut bit_reader0, table, &mut ptr0, state0);
                state1 = step(&mut bit_reader1, table, &mut ptr1, state1);
                state2 = step(&mut bit_reader2, table, &mut ptr2, state2);
                state3 = step(&mut bit_reader3, table, &mut ptr3, state3);
            }
        }

        // There are between 1 and 7 unbuffered bytes remaining and buffer is empty.
        // Drain the unbuferred and no more refills will be needed.
        bit_reader0.refill_lookahead();
        bit_reader1.refill_lookahead();
        bit_reader2.refill_lookahead();
        bit_reader3.refill_lookahead();

        // Generate 7 unrolled blocks, one for each size reachable via a jump table
        let bytes_remaining = bit_reader0.bytes_remaining();
        if bytes_remaining == 7 {
            for _ in 0..7 {
                state0 = step(&mut bit_reader0, table, &mut ptr0, state0);
                state1 = step(&mut bit_reader1, table, &mut ptr1, state1);
                state2 = step(&mut bit_reader2, table, &mut ptr2, state2);
                state3 = step(&mut bit_reader3, table, &mut ptr3, state3);
            }
        } else if bytes_remaining == 6 {
            for _ in 0..6 {
                state0 = step(&mut bit_reader0, table, &mut ptr0, state0);
                state1 = step(&mut bit_reader1, table, &mut ptr1, state1);
                state2 = step(&mut bit_reader2, table, &mut ptr2, state2);
                state3 = step(&mut bit_reader3, table, &mut ptr3, state3);
            }
        } else if bytes_remaining == 5 {
            for _ in 0..5 {
                state0 = step(&mut bit_reader0, table, &mut ptr0, state0);
                state1 = step(&mut bit_reader1, table, &mut ptr1, state1);
                state2 = step(&mut bit_reader2, table, &mut ptr2, state2);
                state3 = step(&mut bit_reader3, table, &mut ptr3, state3);
            }
        } else if bytes_remaining == 4 {
            for _ in 0..4 {
                state0 = step(&mut bit_reader0, table, &mut ptr0, state0);
                state1 = step(&mut bit_reader1, table, &mut ptr1, state1);
                state2 = step(&mut bit_reader2, table, &mut ptr2, state2);
                state3 = step(&mut bit_reader3, table, &mut ptr3, state3);
            }
        } else if bytes_remaining == 3 {
            for _ in 0..3 {
                state0 = step(&mut bit_reader0, table, &mut ptr0, state0);
                state1 = step(&mut bit_reader1, table, &mut ptr1, state1);
                state2 = step(&mut bit_reader2, table, &mut ptr2, state2);
                state3 = step(&mut bit_reader3, table, &mut ptr3, state3);
            }
        } else if bytes_remaining == 2 {
            for _ in 0..2 {
                state0 = step(&mut bit_reader0, table, &mut ptr0, state0);
                state1 = step(&mut bit_reader1, table, &mut ptr1, state1);
                state2 = step(&mut bit_reader2, table, &mut ptr2, state2);
                state3 = step(&mut bit_reader3, table, &mut ptr3, state3);
            }
        } else if bytes_remaining == 1 {
            state0 = step(&mut bit_reader0, table, &mut ptr0, state0);
            state1 = step(&mut bit_reader1, table, &mut ptr1, state1);
            state2 = step(&mut bit_reader2, table, &mut ptr2, state2);
            state3 = step(&mut bit_reader3, table, &mut ptr3, state3);
        }

        state0 = converge(
            bytes1,
            state0,
            state1,
            &mut ptr0,
            &mut ptr1,
            &mut decoded1,
            table,
        );

        state0 = converge(
            bytes2,
            state0,
            state2,
            &mut ptr0,
            &mut ptr2,
            &mut decoded2,
            table,
        );

        state0 = converge(
            bytes3,
            state0,
            state3,
            &mut ptr0,
            &mut ptr3,
            &mut decoded3,
            table,
        );

        if let Some(last_bytes) = tail {
            for byte in last_bytes {
                let symbols: &[u8; 9] = table
                    .tables
                    .get_unchecked(state0)
                    .symbols
                    .get_unchecked(*byte as usize);

                state0 = symbols[0] as usize;
                let src_ptr = symbols.as_ptr().add(1);
                std::ptr::copy_nonoverlapping(src_ptr, ptr0, 8);

                let symbols = symbols.last_chunk::<8>().unwrap();
                let symbol_block = u64::from_le_bytes(*symbols);
                let len = 8 - (symbol_block.leading_zeros() / 8) as usize;
                ptr0 = ptr0.add(len);
            }
        }

        let final_len = ptr0.offset_from(decoded0.as_ptr()) as usize;
        decoded0.set_len(final_len);

        let slice = std::slice::from_raw_parts(decoded0.as_ptr(), final_len);
        let mut decoded = std::str::from_utf8_unchecked(slice).to_owned();
        decoded.truncate(packet.decoded_bytes_len as usize);
        decoded
    }
}

// removing inline(always) from here when ones below it didn't have it didn't really alter any time.
#[inline(always)]
unsafe fn step_state(
    bit_reader: &mut BigEndianReader,
    table: &StateTables,
    write_ptr: &mut *mut u8,
    state: usize,
) -> usize {
    let index = bit_reader.peek(8) as usize;
    let symbols: &[u8; 9] = table
        .tables
        .get_unchecked(state)
        .symbols
        .get_unchecked(index);

    let next_state = symbols[0] as usize;
    bit_reader.consume(8);

    let symbols = symbols.last_chunk::<8>().unwrap();
    let symbol_block = u64::from_le_bytes(*symbols);
    let len = 8 - (symbol_block.leading_zeros() / 8) as usize;
    *write_ptr = write_ptr.add(len);

    next_state
}

// Removing inline(always) from here while funcs above it have it reduced time from 89 (see next funct)
// to 26.09
unsafe fn step(
    bit_reader: &mut BigEndianReader,
    table: &StateTables,
    write_ptr: &mut *mut u8,
    state: usize,
) -> usize {
    let index = bit_reader.peek(8) as usize;
    let symbols: &[u8; 9] = table
        .tables
        .get_unchecked(state)
        .symbols
        .get_unchecked(index);

    let next_state = symbols[0] as usize;
    bit_reader.consume(8);

    let symbols = symbols.last_chunk::<8>().unwrap();
    symbols
        .iter()
        .enumerate()
        .for_each(|(i, x)| *write_ptr.add(i) = *x);

    let symbol_block = u64::from_le_bytes(*symbols);
    let len = 8 - (symbol_block.leading_zeros() / 8) as usize;
    *write_ptr = write_ptr.add(len);

    next_state
}

// Removing inline(always) from here while the funcs above have inline improves time from 120 to 89
// putting the inline back when step and step state didn't have inline improved time from 26.09 to 25.29 (see above)
#[inline(always)]
unsafe fn converge(
    bytes1: &[u8],
    mut state0: usize,
    mut state1: usize,
    ptr0: &mut *mut u8,
    ptr1: &mut *mut u8,
    decoded1: &mut [u8],
    table: &StateTables,
) -> usize {
    let mut bit_reader0 = BigEndianReader::new(bytes1);
    let mut bit_reader1 = BigEndianReader::new(bytes1);

    let prev_state1 = state1;
    let mut ptr1_reset = decoded1.as_mut_ptr();
    state1 = 0;

    while bit_reader0.unbuffered_bytes_remaining() > 0 && state0 != state1 {
        bit_reader0.refill_lookahead();
        bit_reader1.refill_lookahead();
        state0 = step(&mut bit_reader0, table, ptr0, state0);
        state1 = step_state(&mut bit_reader1, table, &mut ptr1_reset, state1);
    }
    while bit_reader0.bytes_remaining() > 0 && state0 != state1 {
        state0 = step(&mut bit_reader0, table, ptr0, state0);
        state1 = step_state(&mut bit_reader1, table, &mut ptr1_reset, state1);
    }
    if state0 != state1 {
        return state0;
    }

    let remaining_len = ptr1.offset_from(ptr1_reset) as usize;
    ptr0.copy_from_nonoverlapping(ptr1_reset, remaining_len);
    *ptr0 = ptr0.add(remaining_len);

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
        unsafe { *table_indices.get_unchecked(node.index.unwrap()) }
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
