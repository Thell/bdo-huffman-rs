use common::min_heap::*;
use common::packet::Packet;

use bitter::{BigEndianReader, BitReader};

const MAX_TREE_LEN: usize = 23;

pub fn decode_packet(content: &[u8]) -> String {
    let packet = &Packet::new(content);
    let mut tree = [HeapNode::default(); MAX_TREE_LEN];
    huffman_tree(packet, &mut tree);
    let (max_depth, table) = &symbol_table(&tree);
    decode_message(packet, *max_depth as u32, table)
}

fn decode_message(packet: &Packet, peek_count: u32, table: &[(u8, u8)]) -> String {
    // Add slop space instead of checking write_index against decoded_len.
    let mut decoded: Vec<u8> = vec![0; packet.decoded_bytes_len as usize + 8];
    let mut write_index = 0usize;

    let mut bit_reader = BigEndianReader::new(packet.encoded_message);

    // Lookahead is 56bits
    // Consume unbuffered bytes; guaranteed 7 8-bit indices per iteration.
    // Since each lookup is not guaranteed to consume all bits try processing more.
    while bit_reader.unbuffered_bytes_remaining() > 7 {
        bit_reader.refill_lookahead();
        for _ in 0..8 {
            let index = bit_reader.peek(peek_count);
            let (bits_used, symbol) = table[index as usize];
            bit_reader.consume(bits_used as u32);
            decoded[write_index] = symbol;
            write_index += 1;
        }
        while bit_reader.lookahead_bits() >= 8 {
            let index = bit_reader.peek(peek_count);
            let (bits_used, symbol) = table[index as usize];
            bit_reader.consume(bits_used as u32);
            decoded[write_index] = symbol;
            write_index += 1;
        }
    }

    // Drain unbuffered bytes with safe refill.
    while bit_reader.unbuffered_bytes_remaining() > 0 {
        bit_reader.refill_lookahead();
        let index = bit_reader.peek(peek_count);
        let (bits_used, symbol) = table[index as usize];
        bit_reader.consume(bits_used as u32);
        decoded[write_index] = symbol;
        write_index += 1;
    }

    // Consume lookahead without refill or peek checks until the last byte.
    while bit_reader.has_bits_remaining(8) {
        let index = bit_reader.peek(peek_count);
        let (bits_used, symbol) = table[index as usize];
        bit_reader.consume(bits_used as u32);
        decoded[write_index] = symbol;
        write_index += 1;
    }

    // Drain partial byte remaining bits with peek checks.
    // should also use  `&& write_index < packet.decoded_bytes_len as usize` for equality with
    // the safe and _ptr versions.
    while bit_reader.has_bits_remaining(1) && write_index < packet.decoded_bytes_len as usize {
        let lookahead_count = bit_reader.lookahead_bits().min(peek_count);
        let last_bits = bit_reader.peek(lookahead_count);
        let index = (last_bits << (peek_count - lookahead_count)) as usize;

        let (bits_used, symbol) = table[index];
        bit_reader.consume(bits_used as u32);
        decoded[write_index] = symbol;
        write_index += 1;
    }

    // Truncate decoded slop.
    let slice = &decoded[..];
    let mut decoded = std::str::from_utf8(slice).unwrap().to_owned();
    decoded.truncate(packet.decoded_bytes_len as usize);
    decoded
}

#[inline(always)]
fn process_heap_node(node: &HeapNode, tree: &mut [HeapNode; MAX_TREE_LEN], index: usize) {
    if node.symbol.is_some() {
        tree[index].symbol = node.symbol;
    } else {
        tree[index].left_index = node.left_index;
        tree[index].right_index = node.right_index;
    }
}

fn huffman_tree(packet: &Packet, tree: &mut [HeapNode; MAX_TREE_LEN]) {
    // Set the root node.
    tree[0].symbol = None;
    tree[0].left_index = 1;
    tree[0].right_index = 2;

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

#[inline(never)]
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

fn symbol_table(tree: &[HeapNode; MAX_TREE_LEN]) -> (u8, Vec<(u8, u8)>) {
    let mut codes = [0u8; MAX_TREE_LEN];
    let mut depths = [0u8; MAX_TREE_LEN];
    let mut max_depth = 0;

    for index in 0..MAX_TREE_LEN {
        let node = &tree[index];
        if node.symbol.is_none() {
            let depth = depths[index] + 1;
            max_depth = max_depth.max(depth);

            depths[node.left_index as usize] = depth;
            codes[node.left_index as usize] = codes[index] << 1;

            depths[node.left_index as usize + 1] = depth;
            codes[node.left_index as usize + 1] = (codes[index] << 1) | 1;
        }
    }

    let num_entries = 1 << max_depth; // 2^max_depth entries
    let mut code_table = vec![(0u8, 0u8); num_entries];

    for index in 0..MAX_TREE_LEN {
        let node = &tree[index];
        if let Some(symbol) = node.symbol {
            let depth = depths[index];

            let shift = max_depth - depth;
            let range_start = (codes[index] as usize) << shift;
            let range_len = 1 << shift;
            let range_end = range_start + range_len;

            code_table[range_start..range_end].fill((depth, symbol));
        }
    }

    (max_depth, code_table)
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct HeapNode {
    left_index: u8,
    right_index: u8,
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
            left_index: 0,
            right_index: 0,
            symbol,
            frequency,
        }
    }
    fn new_parent(frequency: u32, tree_index: u8) -> Self {
        Self {
            left_index: tree_index,
            right_index: tree_index + 1,
            symbol: None,
            frequency,
        }
    }
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
        for case in ALL_CASES {
            let content = case.request();
            let packet = &Packet::new(&content);
            let mut tree = [super::HeapNode::default(); super::MAX_TREE_LEN];
            super::huffman_tree(packet, &mut tree);
            let _ = super::symbol_table(&tree);
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
        let content = case.request();
        let packet = &Packet::new(&content);
        bencher.bench_local(move || {
            let mut tree = [HeapNode::default(); MAX_TREE_LEN];
            huffman_tree(packet, &mut tree);
            black_box(tree);
        });
    }

    #[divan::bench(sample_count = 1_000_000, args = [ALL_CASES[0], ALL_CASES[5]])]
    fn gen_table(bencher: Bencher, case: &Case) {
        let content = case.request();
        let packet = &Packet::new(&content);
        bencher.bench_local(move || {
            let mut tree = [HeapNode::default(); MAX_TREE_LEN];
            huffman_tree(packet, &mut tree);
            black_box(symbol_table(&tree));
        });
    }

    #[divan::bench(args = ALL_CASES)]
    fn decode_message(bencher: Bencher, case: &Case) {
        let content = case.request();
        let packet = &Packet::new(&content);
        let mut tree = [HeapNode::default(); MAX_TREE_LEN];
        huffman_tree(packet, &mut tree);
        let (max_depth, table) = symbol_table(&tree);
        bencher
            .counter(BytesCount::from(packet.decoded_bytes_len))
            .bench_local(move || {
                super::decode_message(black_box(packet), max_depth as u32, &table);
            });
    }

    #[divan::bench(args = ALL_CASES)]
    fn decode_packet(bencher: Bencher, case: &Case) {
        let content = case.request();
        bencher.bench_local(move || {
            super::decode_packet(black_box(&content));
        });
    }
}
