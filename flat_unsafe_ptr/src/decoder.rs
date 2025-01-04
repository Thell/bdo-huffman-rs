use common::min_heap::*;
use common::packet::Packet;

pub fn decode_packet(content: &[u8]) -> String {
    let packet = &Packet::new(content);
    let tree = &huffman_tree(packet);
    unsafe { decode_message(packet, tree) }
}

unsafe fn decode_message(packet: &Packet, tree: &[TreeNode]) -> String {
    let mut decoded: Vec<u8> = Vec::with_capacity(packet.decoded_bytes_len as usize);
    let mut write_index = 0;
    let root = unsafe { tree.get_unchecked(0) };
    let mut node = root;
    let mut read_index = 0;

    'outer: loop {
        let mut bits = *packet.encoded_message.get_unchecked(read_index);
        read_index += 1;

        for _ in 0..8 {
            let direction = ((bits & 0b1000_0000) != 0) as usize;
            bits <<= 1;
            node = (*(&node.left_ptr as *const _ as *const *const TreeNode)
                .add(direction)
                .as_ref()
                .unwrap_unchecked())
            .as_ref()
            .unwrap_unchecked();

            if let Some(symbol) = node.symbol {
                *decoded.as_mut_ptr().add(write_index) = symbol;
                write_index += 1;
                if write_index == packet.decoded_bytes_len as usize {
                    break 'outer;
                }
                node = root;
            }
        }
    }

    decoded.set_len(write_index);
    let slice = std::slice::from_raw_parts(decoded.as_ptr(), decoded.len());
    std::str::from_utf8_unchecked(slice).to_owned()
}

fn huffman_tree(packet: &Packet) -> Vec<TreeNode> {
    let mut heap = unsafe { symbols_heap(packet) };

    let mut right_index = 2 * packet.symbol_count as usize - 1;
    let mut tree = vec![TreeNode::default(); right_index];
    right_index -= 1;

    // Successively move two smallest nodes from heap to tree
    loop {
        let left = heap.pop();
        let right = heap.pop();
        let parent_frequency = left.frequency + right.frequency;

        // Add popped nodes to the tree by setting the existing node values
        tree[right_index - 1].symbol = left.symbol;
        tree[right_index].symbol = right.symbol;
        tree[right_index - 1].left_ptr = &tree[left.left_index as usize] as *const TreeNode;
        tree[right_index].left_ptr = &tree[right.left_index as usize] as *const TreeNode;
        tree[right_index - 1].right_ptr = &tree[left.right_index as usize] as *const TreeNode;
        tree[right_index].right_ptr = &tree[right.right_index as usize] as *const TreeNode;

        if right_index < 3 {
            // Move the last node (the root) to the tree
            tree[0].symbol = None;
            tree[0].left_ptr = &tree[1] as *const TreeNode;
            tree[0].right_ptr = &tree[2] as *const TreeNode;
            break;
        } else {
            // Add a parent node to the heap for ordering
            let parent =
                HeapNode::new_parent(parent_frequency, right_index as u8 - 1, right_index as u8);
            right_index -= 2;
            heap.push(parent);
        }
    }
    tree
}

unsafe fn symbols_heap(packet: &Packet) -> MinHeap<HeapNode> {
    let mut heap = MinHeap::<HeapNode>::new();
    let ptr = packet.symbol_frequency_bytes.as_ptr();
    for i in 0..packet.symbol_count {
        let freq_ptr = ptr.add(i as usize * 8) as *const u32;
        let symbol_ptr = ptr.add(i as usize * 8 + 4);

        let frequency = freq_ptr.read_unaligned();
        let symbol = symbol_ptr.read();
        heap.push(HeapNode::new(Some(symbol), frequency));
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
            left_index: 0,
            right_index: 0,
            symbol,
            frequency,
        }
    }
    fn new_parent(frequency: u32, left_index: u8, right_index: u8) -> Self {
        Self {
            left_index,
            right_index,
            symbol: None,
            frequency,
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
        let tree = huffman_tree(packet);
        bencher
            .counter(BytesCount::from(packet.decoded_bytes_len))
            .bench_local(move || {
                unsafe { super::decode_message(black_box(&packet), &tree) };
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
