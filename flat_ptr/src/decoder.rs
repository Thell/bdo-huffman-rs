use common::min_heap::*;
use common::packet::Packet;

pub fn decode_packet(content: &[u8]) -> String {
    let packet = &Packet::new(content);
    let tree = tree(packet);
    decode_message(packet, &tree)
}

#[allow(clippy::unnecessary_cast)]
fn decode_message(packet: &Packet, tree: &[TreeNode]) -> String {
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
                true => unsafe { &*node.right_ptr },
                false => unsafe { &*node.left_ptr },
            };

            if let Some(symbol) = node.symbol {
                decoded[write_index] = symbol;
                write_index += 1;
                if write_index == packet.decoded_bytes_len as usize {
                    break 'outer;
                }
                node = root;
            }
        }
    }

    let slice = &decoded[..];
    std::str::from_utf8(slice).unwrap().to_owned()
}

fn tree(packet: &Packet) -> Vec<TreeNode> {
    let mut heap = symbols_heap(packet);

    let mut right_index = 2 * packet.symbol_count as usize - 1;
    let mut tree = vec![TreeNode::default(); right_index];
    right_index -= 1;

    loop {
        // Move two smallest nodes from heap to vec ensuring smallest on the left
        let left = heap.pop();
        let right = heap.pop();
        let parent_frequency = left.frequency + right.frequency;

        // Add popped nodes to the tree vec by setting the existing node values
        tree[right_index - 1].symbol = left.symbol;
        tree[right_index].symbol = right.symbol;
        tree[right_index - 1].left_ptr = &tree[left.left_index as usize] as *const TreeNode;
        tree[right_index].left_ptr = &tree[right.left_index as usize] as *const TreeNode;
        tree[right_index - 1].right_ptr = &tree[left.right_index as usize] as *const TreeNode;
        tree[right_index].right_ptr = &tree[right.right_index as usize] as *const TreeNode;

        // Add a parent node to the heap for ordering
        heap.push(HeapNode::new_parent(
            parent_frequency,
            right_index as u8 - 1,
            right_index as u8,
        ));

        right_index -= 2;
        if right_index < 2 {
            // Move the last node (the root) to the tree vec
            let root = heap.pop();
            tree[0].symbol = root.symbol;
            tree[0].left_ptr = &tree[root.left_index as usize] as *const TreeNode;
            tree[0].right_ptr = &tree[root.right_index as usize] as *const TreeNode;
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

#[derive(PartialEq, Eq)]
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

#[derive(Clone)]
struct TreeNode {
    left_ptr: *const TreeNode,
    right_ptr: *const TreeNode,
    symbol: Option<u8>,
}

impl Default for TreeNode {
    fn default() -> Self {
        Self {
            left_ptr: std::ptr::null(),
            right_ptr: std::ptr::null(),
            symbol: None,
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
