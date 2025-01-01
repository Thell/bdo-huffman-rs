use common::min_heap::*;
use common::packet::Packet;

pub fn decode_packet(content: &[u8]) -> String {
    let packet = &Packet::new(content);
    let tree = tree(packet);
    decode_message(packet, &tree)
}

fn decode_message(packet: &Packet, tree: &[HeapNode]) -> String {
    let mut decoded: Vec<u8> = vec![0; packet.decoded_bytes_len as usize];
    let root = &tree[0];
    let mut node = root;
    let mut write_index = 0;

    'outer: for byte in packet.encoded_message.iter() {
        let mut bits = *byte;

        for _ in 0..8 {
            let direction = (bits & 0b1000_0000) != 0;
            bits <<= 1;

            node = match direction {
                true => &tree[node.right_index as usize],
                false => &tree[node.left_index as usize],
            };

            if let Some(symbol) = node.symbol {
                decoded[write_index] = symbol;
                write_index += 1;
                node = root;

                if write_index == packet.decoded_bytes_len as usize {
                    break 'outer;
                }
            }
        }
    }

    let slice = &decoded[..];
    std::str::from_utf8(slice).unwrap().to_owned()
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

use common::min_heap::MinHeapNode;

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
            .bench_local(move || {
                super::decode_message(black_box(&packet), &tree);
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
