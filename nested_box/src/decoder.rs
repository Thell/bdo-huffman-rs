use common::min_heap::*;
use common::packet::Packet;

pub fn decode_packet(content: &[u8]) -> String {
    let packet = &Packet::new(content);
    let tree = &huffman_tree(packet);
    decode_message(packet, tree)
}

fn decode_message(packet: &Packet, tree: &HeapNode) -> String {
    let mut decoded: Vec<u8> = vec![0; packet.decoded_bytes_len as usize];
    let mut write_index = 0;
    let mut current = tree;

    'outer: for byte in packet.encoded_message.iter() {
        let mut bits = *byte;

        for _ in 0..8 {
            let direction = (bits & 0b1000_0000) != 0;
            bits <<= 1;

            current = match direction {
                true => current.right_child.as_ref().unwrap(),
                false => current.left_child.as_ref().unwrap(),
            };

            if let Some(symbol) = current.symbol {
                decoded[write_index] = symbol;
                write_index += 1;
                if write_index == packet.decoded_bytes_len as usize {
                    break 'outer;
                }
                current = tree;
            }
        }
    }

    let slice = &decoded[..];
    std::str::from_utf8(slice).unwrap().to_owned()
}

fn huffman_tree(packet: &Packet) -> HeapNode {
    let mut heap = symbols_heap(packet);
    let mut size = heap.len();

    // Successively move two smallest nodes from heap to tree
    while size > 1 {
        let left = heap.pop();
        let right = heap.pop();

        // Add a parent node to the heap for ordering
        heap.push(HeapNode::new_parent(left, right));
        size -= 1;
    }
    // Return the last node (the root) as the tree
    heap.pop()
}

fn symbols_heap(packet: &Packet) -> MinHeap<HeapNode> {
    let mut heap = MinHeap::<HeapNode>::new();
    let bytes = &packet.symbol_frequency_bytes;
    for i in 0..packet.symbol_count {
        let pos = (i as usize) * 8;
        let frequency = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap());
        let symbol = bytes[pos + 4];
        heap.push(HeapNode::new(Some(symbol), frequency));
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
        let tree = huffman_tree(packet);
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
