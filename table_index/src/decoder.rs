use common::min_heap::*;
use common::packet::Packet;

use bitter::{BigEndianReader, BitReader};

pub fn decode_packet(content: &[u8]) -> String {
    let packet = &Packet::new(content);
    let tree = &tree(packet);
    let table = &table(tree);
    decode_message(packet, &table)
}

fn decode_message(packet: &Packet, table: &SymbolTable) -> String {
    // Add slop space instead of checking write_index against decoded_len.
    let mut decoded: Vec<u8> = vec![0; packet.decoded_bytes_len as usize + 8];
    let mut write_index = 0usize;

    let mut bits = BigEndianReader::new(packet.encoded_message);

    // Lookahead is 56bits
    // Consume unbuffered bytes by processing 7 8-bit indices per iteration.
    // This does not consume all bits in lookahead on each iteration.
    while bits.unbuffered_bytes_remaining() > 7 {
        bits.refill_lookahead();
        lookup_unchecked_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
        lookup_unchecked_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
        lookup_unchecked_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
        lookup_unchecked_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
        lookup_unchecked_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
        lookup_unchecked_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
        lookup_unchecked_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
    }

    // Drain unbuffered bytes with safe refill.
    while bits.unbuffered_bytes_remaining() > 0 {
        bits.refill_lookahead();
        lookup_unchecked_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
    }

    // Consume lookahead without refill or peek checks until the last byte.
    while bits.has_bits_remaining(8) {
        lookup_unchecked_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
    }

    // Drain partial byte bits with peek checks.
    while bits.has_bits_remaining(1) && write_index < packet.decoded_bytes_len as usize {
        lookup_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
    }

    // Truncate decoded slop.
    let slice = &decoded[..];
    let mut decoded = std::str::from_utf8(slice).unwrap().to_owned();
    decoded.truncate(packet.decoded_bytes_len as usize);
    decoded
}

fn lookup_unchecked_prefix_table_safe(
    bits: &mut BigEndianReader,
    table: &SymbolTable,
    write_index: &mut usize,
    decoded: &mut [u8],
) {
    let index = bits.peek(8) as usize;

    let symbols = &table.symbols[index];
    let used_bits = table.bits_used[index];

    get_symbols_unchecked_safe(symbols, write_index, decoded);
    bits.consume(used_bits as u32);
}

fn lookup_prefix_table_safe(
    bits: &mut BigEndianReader,
    table: &SymbolTable,
    write_index: &mut usize,
    decoded: &mut [u8],
) {
    let lookahead_count = bits.lookahead_bits().min(8);
    let last_bits = bits.peek(lookahead_count);
    let index = (last_bits << (8 - lookahead_count)) as usize;

    let symbols = &table.symbols[index];
    let used_bits = table.bits_used[index];

    get_symbols_unchecked_safe(symbols, write_index, decoded);

    let bits_to_consume = lookahead_count.min(used_bits as u32);
    bits.consume(bits_to_consume);
}

fn get_symbols_unchecked_safe(symbols: &[u8], write_index: &mut usize, decoded: &mut [u8]) {
    decoded[*write_index] = symbols[0];
    *write_index += 1;
    for i in 1..6 {
        if symbols[i] > 0 {
            decoded[*write_index] = symbols[i];
            *write_index += 1;
        } else {
            break;
        }
    }
}

fn tree(packet: &Packet) -> Vec<HeapNode> {
    let mut heap = symbols_heap(packet);

    let mut right_index = 2 * packet.symbol_count as usize - 1;
    let mut tree = vec![HeapNode::default(); right_index];
    right_index -= 1;

    loop {
        let left = heap.pop();
        let right = heap.pop();
        let parent_frequency = left.frequency + right.frequency;

        tree[right_index - 1] = left;
        tree[right_index] = right;

        heap.push(HeapNode::new_parent(
            parent_frequency,
            right_index as u8 - 1,
            right_index as u8,
        ));

        right_index -= 2;
        if right_index < 2 {
            tree[0] = heap.pop();
            break;
        }
    }
    tree
}

fn symbols_heap(packet: &Packet) -> MinHeap<HeapNode> {
    let mut heap = MinHeap::<HeapNode>::new();
    let ptr = packet.symbol_table_bytes.as_ptr();
    unsafe {
        for i in 0..packet.symbol_count {
            let freq_ptr = ptr.add(i as usize * 8) as *const u32;
            let symbol_ptr = ptr.add(i as usize * 8 + 4);

            let frequency = freq_ptr.read_unaligned();
            let symbol = symbol_ptr.read();
            heap.push(HeapNode::new(Some(symbol), frequency));
        }
    }
    heap
}

#[repr(C)]
struct SymbolTable {
    bits_used: [u8; 256],
    symbols: [[u8; 6]; 256],
}

impl Default for SymbolTable {
    fn default() -> Self {
        SymbolTable {
            bits_used: [0u8; 256],
            symbols: [[0u8; 6]; 256],
        }
    }
}

fn table(tree: &[HeapNode]) -> SymbolTable {
    // Generate a multi-symbol lookup table.
    // Decodes all 8 step paths through the tree storing each symbol visited,
    // the number of symbols written, and the number of bits used when the
    // last symbol was visited.
    let mut table = SymbolTable::default();
    let root = &tree[0];

    for byte in 0u8..=255 {
        let symbols = &mut table.symbols[byte as usize];
        let mut node = root;
        let mut bits = byte;
        let mut bits_used = 0;
        let mut write_index = 0;

        for i in 0..8 {
            let direction = (bits & 0b1000_0000) != 0;
            bits <<= 1;

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
        }
        table.bits_used[byte as usize] = bits_used;
    }
    table
}

#[derive(Clone, Default, PartialEq, Eq)]
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
            symbol,
            frequency,
            left_index: 0,
            right_index: 0,
        }
    }
    fn new_parent(frequency: u32, left_index: u8, right_index: u8) -> Self {
        Self {
            symbol: None,
            frequency,
            left_index,
            right_index,
        }
    }
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

    #[divan::bench(args = ALL_CASES)]
    fn decode_message(bencher: Bencher, case: &Case) {
        let response_bytes = case.request();
        let packet = &Packet::new(&response_bytes);
        let tree = &tree(packet);
        let table = table(tree);
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
}
