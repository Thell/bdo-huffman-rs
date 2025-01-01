use bit_vec::BitVec;
use common::min_heap::*;
use common::packet::Packet;

pub fn decode_packet(content: &[u8]) -> String {
    let packet = &Packet::new(content);
    let tree = &tree(packet);
    decode_message(packet, tree)
}

fn decode_message(packet: &Packet, tree: &HeapNode) -> String {
    let decoded_len = packet.decoded_bytes_len;
    let mut decoded = String::with_capacity(decoded_len as usize);
    let mut current = tree;

    let mut bits = BitVec::from_bytes(packet.encoded_message);
    bits.truncate(packet.bitstream_len as usize);

    for bit in bits.iter() {
        current = if bit {
            current
                .right_child
                .as_deref()
                .expect("Should have right child!")
        } else {
            current
                .left_child
                .as_deref()
                .expect("Should have left child!")
        };

        if let Some(symbol) = current.symbol {
            decoded.push(symbol as char);
            current = tree;
        }
    }
    decoded
}

fn tree(packet: &Packet) -> HeapNode {
    let mut heap = symbols_heap(packet);
    let mut size = heap.len();

    while size > 1 {
        let left = heap.pop();
        let right = heap.pop();
        heap.push(HeapNode::new_parent(left, right));
        size -= 1;
    }
    heap.pop()
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

#[derive(Clone, Eq, PartialEq)]
struct HeapNode {
    symbol: Option<u8>,
    frequency: u32,
    left_child: Option<Box<HeapNode>>,
    right_child: Option<Box<HeapNode>>,
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
    fn new(symbol: Option<u8>, freq: u32) -> Self {
        Self {
            symbol,
            frequency: freq,
            left_child: None,
            right_child: None,
        }
    }
    fn new_parent(left: Self, right: Self) -> Self {
        HeapNode {
            symbol: None,
            frequency: left.frequency + right.frequency,
            left_child: Some(Box::new(left)),
            right_child: Some(Box::new(right)),
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

#[divan::bench_group(sample_count = common::test_cases::BENCH_SAMPLE_COUNT)]
mod bench {
    use super::*;
    use common::test_cases::*;

    use divan::counter::BytesCount;
    use divan::{black_box, Bencher};

    #[divan::bench(args = ALL_CASES)]
    fn decode_message(bencher: Bencher, case: &Case) {
        let response_bytes = case.request();
        let packet = &Packet::new(&response_bytes);
        let tree = tree(packet);
        bencher
            .counter(BytesCount::from(packet.decoded_bytes_len))
            .bench_local(move || black_box(super::decode_message(black_box(&packet), &tree)));
    }

    #[divan::bench(args = ALL_CASES)]
    fn decode_packet(bencher: Bencher, case: &Case) {
        let content = case.request();
        bencher.bench_local(move || black_box(super::decode_packet(black_box(&content))));
    }
}
