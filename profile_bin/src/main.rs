use common::test_cases::*;
use std::hint::black_box;

fn main() {
    let content = ALL_CASES[1].request(); // this is the large medium case
    for _ in 0..1_000_000 {
        let result = decode_packet(black_box(&content));
        black_box(result);
    }
}

// NOTE: Copy some crate's decoder code here and put `` where needed for profiling.

// state_table_unsafe
use bitter::{BigEndianReader, BitReader};
use common::min_heap::*;
use common::packet::Packet;

const MAX_TREE_LEN: usize = 23;

pub fn decode_packet(content: &[u8]) -> String {
    let packet = &Packet::new(content);
    let mut tree = [HeapNode::default(); MAX_TREE_LEN];
    huffman_tree(packet, &mut tree);
    let table = &state_tables(&tree);
    decode_message(packet, table)
}

// fn decode_message(packet: &Packet, table: &StateTables) -> String {
//     // Add slop space instead of checking write_index against decoded_len.
//     let mut decoded: Vec<u8> = vec![0; packet.decoded_bytes_len as usize + 8];
//     let mut write_index = 0usize;
//     let mut state = 0;

//     for &byte in packet.encoded_message {
//         decoded[write_index] = table.tables[state].symbols[byte as usize][0];
//         write_index += 1;
//         for i in 1..8 {
//             if table.tables[state].symbols[byte as usize][i] > 0 {
//                 decoded[write_index] = table.tables[state].symbols[byte as usize][i];
//                 write_index += 1;
//             } else {
//                 break;
//             }
//         }
//         state = table.tables[state].symbols[byte as usize][8] as usize;
//     }

//     // Truncate decoded slop.
//     unsafe {
//         std::str::from_utf8_unchecked(&decoded[..packet.decoded_bytes_len as usize]).to_owned()
//     }
// }

fn decode_message(packet: &Packet, table: &StateTables) -> String {
    // Add slop space instead of checking write_index against decoded_len.
    let mut decoded: Vec<u8> = Vec::with_capacity(packet.decoded_bytes_len as usize + 8);
    let mut index = 0usize;
    let mut state = 0;

    let mut bit_reader = BigEndianReader::new(packet.encoded_message);

    // Lookahead is 56bits
    // Consume unbuffered bytes; guaranteed 7 8-bit indices per iteration.
    // Since each lookup is not guaranteed to consume all bits try processing more.
    while bit_reader.unbuffered_bytes_remaining() > 7 {
        unsafe {
            bit_reader.refill_lookahead_unchecked();
            state = lookup_byte(&mut bit_reader, table, &mut index, &mut decoded, state);
            state = lookup_byte(&mut bit_reader, table, &mut index, &mut decoded, state);
            state = lookup_byte(&mut bit_reader, table, &mut index, &mut decoded, state);
            state = lookup_byte(&mut bit_reader, table, &mut index, &mut decoded, state);
            state = lookup_byte(&mut bit_reader, table, &mut index, &mut decoded, state);
            state = lookup_byte(&mut bit_reader, table, &mut index, &mut decoded, state);
            state = lookup_byte(&mut bit_reader, table, &mut index, &mut decoded, state);
            // Since the checked `refill_lookahead` is more expensive than the lookup
            // this improves performance on medium_small+ sized msgs.
            while bit_reader.lookahead_bits() >= 8 {
                state = lookup_byte(&mut bit_reader, table, &mut index, &mut decoded, state);
            }
        }
    }

    // Drain unbuffered bytes with safe refill.
    while bit_reader.unbuffered_bytes_remaining() > 0 {
        bit_reader.refill_lookahead();
        state = unsafe { lookup_byte(&mut bit_reader, table, &mut index, &mut decoded, state) };
    }

    // Consume lookahead without refill or peek checks until the last byte.
    while bit_reader.has_bits_remaining(8) {
        state = unsafe { lookup_byte(&mut bit_reader, table, &mut index, &mut decoded, state) };
    }

    // Drain partial byte remaining bits with peek checks.
    // should also use  `&& write_index < packet.decoded_bytes_len as usize` for equality with
    // the safe and _ptr versions.
    while bit_reader.has_bits_remaining(1) {
        state = unsafe {
            lookup_bits_unchecked(&mut bit_reader, table, &mut index, &mut decoded, state)
        };
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
unsafe fn lookup_byte(
    bit_reader: &mut BigEndianReader,
    table: &StateTables,
    write_index: &mut usize,
    decoded: &mut [u8],
    state: usize,
) -> usize {
    let index = bit_reader.peek(8) as usize;
    let symbols = table
        .tables
        .get_unchecked(state)
        .symbols
        .get_unchecked(index);
    let state = *symbols.get_unchecked(symbols.len() - 1);
    copy_symbols_unchecked(symbols, write_index, decoded);
    bit_reader.consume(8);
    state as usize
}

#[inline(always)]
unsafe fn lookup_bits_unchecked(
    bit_reader: &mut BigEndianReader,
    table: &StateTables,
    write_index: &mut usize,
    decoded: &mut [u8],
    state: usize,
) -> usize {
    let lookahead_count = bit_reader.lookahead_bits().min(8);
    let last_bits = bit_reader.peek(lookahead_count);
    let index = bit_reader.peek(8) as usize;
    let symbols = table
        .tables
        .get_unchecked(state)
        .symbols
        .get_unchecked(index);
    let state = *symbols.get_unchecked(symbols.len() - 1);

    copy_symbols_unchecked(symbols, write_index, decoded);

    let bits_to_consume = lookahead_count.min(last_bits as u32);
    bit_reader.consume(bits_to_consume);
    state as usize
}

#[inline(always)]
unsafe fn copy_symbols_unchecked(symbols: &[u8], write_index: &mut usize, decoded: &mut [u8]) {
    *decoded.as_mut_ptr().add(*write_index) = *symbols.get_unchecked(0);
    *write_index += 1;
    for i in 1..8 {
        if *symbols.get_unchecked(i) > 0 {
            *decoded.as_mut_ptr().add(*write_index) = *symbols.get_unchecked(i);
            *write_index += 1;
        } else {
            break;
        }
    }
}

fn huffman_tree(packet: &Packet, tree: &mut [HeapNode; MAX_TREE_LEN]) {
    let mut heap = symbols_heap(packet);
    let mut right_index = 2 * packet.symbol_count as usize - 2;

    // Successively move two smallest nodes from heap to tree
    loop {
        let left = heap.pop();
        let right = heap.pop();
        let parent_frequency = left.frequency + right.frequency;

        // Add popped nodes to the tree by setting the existing node values
        tree[right_index - 1].symbol = left.symbol;
        tree[right_index].symbol = right.symbol;
        tree[right_index - 1].left_index = left.left_index;
        tree[right_index].left_index = right.left_index;
        tree[right_index - 1].right_index = left.right_index;
        tree[right_index].right_index = right.right_index;
        tree[right_index - 1].index = Some(right_index - 1);
        tree[right_index].index = Some(right_index);

        if right_index < 3 {
            // Move the last node (the root) to the tree
            tree[0].symbol = None;
            tree[0].left_index = 1;
            tree[0].right_index = 2;
            tree[0].index = Some(0);
            break;
        } else {
            // Add a parent node to the heap for ordering
            let parent =
                HeapNode::new_parent(parent_frequency, right_index as u8 - 1, right_index as u8);
            right_index -= 2;
            heap.push(parent);
        }
    }
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

#[repr(align(64))]
struct StateTables {
    tables: Vec<SymbolTable>,
}

#[inline(always)]
fn state_tables(tree: &[HeapNode; MAX_TREE_LEN]) -> StateTables {
    let (table_indices, child_states, internal_count) = child_states(tree);

    let mut state_tables = StateTables {
        tables: vec![SymbolTable::default(); internal_count as usize],
    };

    let reference_index = child_states.iter().position(|&x| x == 3).unwrap();
    let reference = gen_full_range(
        &tree[reference_index],
        tree,
        &table_indices,
        &SymbolTable::default(),
    );

    for i in 0..MAX_TREE_LEN {
        let table_index = table_indices[i];
        if table_index == MAX_TREE_LEN as u8 {
            continue;
        }
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

#[inline(always)]
fn child_states(tree: &[HeapNode; MAX_TREE_LEN]) -> ([u8; MAX_TREE_LEN], [u8; MAX_TREE_LEN], u8) {
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
    (table_indices, child_states, internal_count)
}

#[inline(always)]
fn copy_lower_gen_upper(
    start_node: &HeapNode,
    tree: &[HeapNode; MAX_TREE_LEN],
    table_indices: &[u8; MAX_TREE_LEN],
    reference_table: &SymbolTable,
) -> SymbolTable {
    let mut table = SymbolTable::default();
    table.symbols[0..=127].copy_from_slice(&reference_table.symbols[0..=127]);
    table.symbols[0..=127]
        .iter_mut()
        .for_each(|x| x[0] = tree[start_node.left_index as usize].symbol.unwrap());

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

#[inline(always)]
fn gen_lower_copy_upper(
    start_node: &HeapNode,
    tree: &[HeapNode; MAX_TREE_LEN],
    table_indices: &[u8; MAX_TREE_LEN],
    reference_table: &SymbolTable,
) -> SymbolTable {
    let mut table = SymbolTable::default();
    table.symbols[128..=255].copy_from_slice(&reference_table.symbols[128..=255]);
    table.symbols[128..=255]
        .iter_mut()
        .for_each(|x| x[0] = tree[start_node.right_index as usize].symbol.unwrap());

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

#[inline(always)]
fn copy_full_range(
    start_node: &HeapNode,
    tree: &[HeapNode; MAX_TREE_LEN],
    _table_indices: &[u8; MAX_TREE_LEN],
    reference_table: &SymbolTable,
) -> SymbolTable {
    let mut table = SymbolTable {
        symbols: reference_table.symbols,
    };
    table.symbols[0..=127]
        .iter_mut()
        .for_each(|x| x[0] = tree[start_node.left_index as usize].symbol.unwrap());
    table.symbols[128..=255]
        .iter_mut()
        .for_each(|x| x[0] = tree[start_node.right_index as usize].symbol.unwrap());
    table
}

#[inline(always)]
fn gen_full_range(
    start_node: &HeapNode,
    tree: &[HeapNode; MAX_TREE_LEN],
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
    mut node: &'a HeapNode,
    symbols: &mut [u8; 9],
    tree: &'a [HeapNode; MAX_TREE_LEN],
    table_indices: &[u8; MAX_TREE_LEN],
) {
    let mut write_index = 0;

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
    symbols[symbols.len() - 1] = if node.symbol.is_some() {
        0
    } else {
        unsafe { *table_indices.get_unchecked(node.index.unwrap()) }
    };
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct HeapNode {
    left_index: u8,
    right_index: u8,
    symbol: Option<u8>,
    frequency: u32,
    index: Option<usize>,
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
            left_index: 1,
            right_index: 2,
            symbol,
            frequency,
            index: None,
        }
    }
    fn new_parent(frequency: u32, left_index: u8, right_index: u8) -> Self {
        Self {
            left_index,
            right_index,
            symbol: None,
            frequency,
            index: None,
        }
    }
}
