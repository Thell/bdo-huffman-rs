use common::min_heap::*;
use common::packet::Packet;

const MAX_TREE_LEN: usize = 23;

pub fn decode_packet(content: &[u8]) -> String {
    let packet = &Packet::new(content);
    let mut tree = [TreeNode::default(); MAX_TREE_LEN];
    huffman_tree(packet, &mut tree);
    decode_message(packet, &tree)
}

fn decode_message(packet: &Packet, tree: &[TreeNode; MAX_TREE_LEN]) -> String {
    let mut decoded: Vec<u8> = vec![0; packet.decoded_bytes_len as usize];
    let mut write_index = 0;
    let root = &tree[0];
    let mut node = root;

    let (last_byte, bytes) = packet.encoded_message.split_last().unwrap();
    for &byte in bytes {
        let mut bits = byte;
        for _ in 0..8 {
            let direction = (bits >> 7) as usize;
            bits <<= 1;
            node = unsafe { step(direction, node) };
            if let Some(symbol) = node.symbol {
                decoded[write_index] = symbol;
                write_index += 1;
                node = root;
            }
        }
    }

    let mut bits = *last_byte;
    for _ in 0..8 {
        let direction = (bits >> 7) as usize;
        bits <<= 1;
        node = unsafe { step(direction, node) };
        if let Some(symbol) = node.symbol {
            decoded[write_index] = symbol;
            write_index += 1;
            if write_index == packet.decoded_bytes_len as usize {
                break;
            }
            node = root;
        }
    }

    let slice = &decoded[..];
    std::str::from_utf8(slice).unwrap().to_owned()
}

#[inline(always)]
unsafe fn step(direction: usize, node: &TreeNode) -> &TreeNode {
    match direction {
        0 => unsafe { &*node.left_ptr },
        _ => unsafe { &*node.right_ptr },
    }
}

#[inline(always)]
fn process_heap_node(node: &HeapNode, tree: &mut [TreeNode; MAX_TREE_LEN], index: usize) {
    unsafe {
        if node.symbol.is_some() {
            tree.get_unchecked_mut(index).symbol = node.symbol;
        } else {
            tree.get_unchecked_mut(index).left_ptr =
                tree.get_unchecked(node.tree_index as usize) as *const TreeNode;
            tree.get_unchecked_mut(index).right_ptr =
                tree.get_unchecked(node.tree_index as usize + 1) as *const TreeNode;
        }
    }
}

fn huffman_tree(packet: &Packet, tree: &mut [TreeNode; MAX_TREE_LEN]) {
    // Set the root node.
    tree[0].symbol = None;
    tree[0].left_ptr = &tree[1] as *const TreeNode;
    tree[0].right_ptr = &tree[2] as *const TreeNode;

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
    for i in 0..packet.symbol_count {
        let pos = (i as usize) * 8;
        let frequency = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap());
        let symbol = bytes[pos + 4];
        heap.push(HeapNode::new(Some(symbol), frequency));
    }
    heap
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

#[derive(Clone, Copy)]
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

    #[divan::bench(args = ALL_CASES)]
    fn decode_message(bencher: Bencher, case: &Case) {
        let content = case.request();
        let packet = &Packet::new(&content);
        let mut tree = [TreeNode::default(); MAX_TREE_LEN];
        huffman_tree(packet, &mut tree);

        bencher
            .counter(BytesCount::from(packet.decoded_bytes_len))
            .bench_local(move || {
                super::decode_message(black_box(packet), &tree);
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
