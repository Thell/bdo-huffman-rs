use crate::node::FlatNode;
use crate::{node::TreeNode, packet::Packet};

use bit_vec::BitVec;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Tree Traversal - Baseline
pub fn treenode_decode_packet_traverse_baseline(content: &[u8]) -> String {
    let packet = Packet::new(content);
    let tree = packet.treenode_tree();
    treenode_decode_message_traverse_baseline(&packet, &tree)
}

pub fn treenode_decode_message_traverse_baseline(packet: &Packet, tree: &TreeNode) -> String {
    let decoded_len = packet.decoded_bytes_len;
    let mut result = String::with_capacity(decoded_len as usize);
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
            result.push(symbol as char);
            current = tree;
        }
    }
    result
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Tree Traversal - TreeNode
pub fn treenode_decode_packet_traverse(content: &[u8]) -> String {
    let packet = Packet::new(content);
    let tree = packet.treenode_tree();
    treenode_decode_message_traverse(&packet, &tree)
}

pub fn treenode_decode_message_traverse(packet: &Packet, tree: &TreeNode) -> String {
    let mut decoded: Vec<u8> = Vec::with_capacity(packet.decoded_bytes_len as usize);
    let mut current = tree;
    let mut read_index = 0;
    let mut write_index = 0;

    'outer: loop {
        let mut bits = unsafe { *packet.encoded_message.get_unchecked(read_index) };
        read_index += 1;

        for _ in 0..8 {
            let bit = (bits & 0b1000_0000) != 0;
            bits <<= 1;

            current = unsafe {
                (*(&current.left_child as *const _ as *const *const TreeNode)
                    .add(bit as usize)
                    .as_ref()
                    .unwrap_unchecked())
                .as_ref()
                .unwrap_unchecked()
            };

            if let Some(symbol) = current.symbol {
                unsafe {
                    *decoded.as_mut_ptr().add(write_index) = symbol;
                }
                write_index += 1;
                current = tree;

                if write_index == packet.decoded_bytes_len as usize {
                    break 'outer;
                }
            }
        }
    }

    unsafe {
        decoded.set_len(write_index);
        let slice = std::slice::from_raw_parts(decoded.as_ptr(), decoded.len());
        std::str::from_utf8_unchecked(slice).to_owned()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Tree Traversal - FlatNode
pub fn flatnode_decode_packet_traverse(content: &[u8]) -> String {
    let packet = Packet::new(content);
    let tree = packet.flatnode_tree();
    flatnode_decode_message_traverse(&packet, &tree)
}

#[allow(clippy::unnecessary_cast)]
pub fn flatnode_decode_message_traverse(packet: &Packet, tree: &[FlatNode]) -> String {
    let mut decoded: Vec<u8> = Vec::with_capacity(packet.decoded_bytes_len as usize);
    let root = unsafe { tree.get_unchecked(0) };
    let mut node = root;
    let mut read_index = 0;
    let mut write_index = 0;

    'outer: loop {
        let mut bits = unsafe { *packet.encoded_message.get_unchecked(read_index) };
        read_index += 1;

        for _ in 0..8 {
            let direction = ((bits & 0b1000_0000) != 0) as usize;
            node = unsafe {
                (*(&node.left_ptr as *const _ as *const *const FlatNode)
                    .add(direction)
                    .as_ref()
                    .unwrap_unchecked())
                .as_ref()
                .unwrap_unchecked()
            };
            bits <<= 1;

            if let Some(symbol) = node.symbol {
                unsafe {
                    *decoded.as_mut_ptr().add(write_index) = symbol;
                }
                write_index += 1;
                node = root;

                if write_index == packet.decoded_bytes_len as usize {
                    break 'outer;
                }
            }
        }
    }

    unsafe {
        decoded.set_len(write_index);
        let slice = std::slice::from_raw_parts(decoded.as_ptr(), decoded.len());
        std::str::from_utf8_unchecked(slice).to_owned()
    }
}

// =========================================================
// MARK: Unit Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_cases::*;

    #[test]
    fn treenode_decode_packet_traverse_baseline() {
        let decoded_message = super::treenode_decode_packet_traverse_baseline(&TEST_BYTES);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn treenode_decode_message_treenode_traverse_baseline() {
        let packet = Packet::new(&TEST_BYTES);
        let tree = packet.treenode_tree();
        let decoded_message = super::treenode_decode_message_traverse_baseline(&packet, &tree);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn treenode_decode_packet_traverse() {
        let decoded_message = super::treenode_decode_packet_traverse(&TEST_BYTES);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn treenode_decode_message_traverse() {
        let packet = Packet::new(&TEST_BYTES);
        let tree = packet.treenode_tree();
        let decoded_message = super::treenode_decode_message_traverse(&packet, &tree);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn all_samples_baseline_vs_treenode_traverse() {
        for case in SAMPLE_CASES {
            println!("case: {}_{}", case.main_category, case.sub_category);
            let content = case.request();
            let expected_result = super::treenode_decode_packet_traverse_baseline(&content);
            let result = super::treenode_decode_packet_traverse(&content);
            assert_eq!(result, expected_result)
        }
    }

    #[test]
    fn flatnode_decode_packet_traverse() {
        let decoded_message = super::flatnode_decode_packet_traverse(&TEST_BYTES);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn flatnode_decode_message_traverse() {
        let packet = Packet::new(&TEST_BYTES);
        let tree = packet.flatnode_tree();
        let decoded_message = super::flatnode_decode_message_traverse(&packet, &tree);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn all_samples_baseline_vs_flatnode_traverse() {
        for case in SAMPLE_CASES {
            println!("case: {}_{}", case.main_category, case.sub_category);
            let content = &case.request();
            let expected_result = super::treenode_decode_packet_traverse_baseline(&content);
            let result = super::flatnode_decode_packet_traverse(&content);
            assert_eq!(result, expected_result)
        }
    }
}

// MARK: Benches

#[divan::bench_group(sample_count = 100_000)]
mod benches_packet {
    use crate::test_cases::*;
    use divan::{black_box, Bencher};

    macro_rules! bench_decode_packet {
        ($name:ident) => {
            #[divan::bench(args = ALL_CASES)]
            fn $name(bencher: Bencher, case: &Case) {
                let response_bytes = case.request();
                bencher.bench_local(move || {
                    super::$name(black_box(&response_bytes));
                });
            }
        };
    }

    bench_decode_packet!(treenode_decode_packet_traverse_baseline);
    bench_decode_packet!(treenode_decode_packet_traverse);
    bench_decode_packet!(flatnode_decode_packet_traverse);
}

#[divan::bench_group(sample_count = 10_000)]
mod benches_message {
    use super::*;
    use crate::test_cases::*;

    use divan::counter::BytesCount;
    use divan::{black_box, Bencher};

    macro_rules! bench_decode_message {
        ($name:ident, $tree:ident) => {
            #[divan::bench(args = ALL_CASES)]
            fn $name(bencher: Bencher, case: &Case) {
                let response_bytes = case.request();
                let packet = Packet::new(&response_bytes);
                let tree = packet.$tree();
                bencher
                    .counter(BytesCount::from(packet.decoded_bytes_len))
                    .bench_local(move || {
                        super::$name(black_box(&packet), &tree);
                    });
            }
        };
    }

    bench_decode_message!(treenode_decode_message_traverse_baseline, treenode_tree);
    bench_decode_message!(treenode_decode_message_traverse, treenode_tree);
    bench_decode_message!(flatnode_decode_message_traverse, flatnode_tree);
}

//////////////////////////////////////////////////////////////////////////////////////////
// Grouped Packet Benches

// Used for each of the size grouped benchmarks.
macro_rules! bench_group_decode_packet {
    ($name:ident) => {
        #[divan::bench()]
        fn $name(bencher: Bencher) {
            let response_bytes = CASE.request();
            bencher.bench_local(move || {
                super::$name(black_box(&response_bytes));
            });
        }
    };
}

#[divan::bench_group(sample_count = 100_000)]
mod decode_packet_group_large {
    use crate::test_cases::*;
    use divan::{black_box, Bencher};

    static CASE: &Case = &ALL_CASES[0];
    bench_group_decode_packet!(treenode_decode_packet_traverse_baseline);
    bench_group_decode_packet!(treenode_decode_packet_traverse);
    bench_group_decode_packet!(flatnode_decode_packet_traverse);
}

#[divan::bench_group(sample_count = 100_000)]
mod decode_packet_group_large_medium {
    use crate::test_cases::*;
    use divan::{black_box, Bencher};

    static CASE: &Case = &ALL_CASES[1];

    bench_group_decode_packet!(treenode_decode_packet_traverse_baseline);
    bench_group_decode_packet!(treenode_decode_packet_traverse);
    bench_group_decode_packet!(flatnode_decode_packet_traverse);
}

#[divan::bench_group(sample_count = 100_000)]
mod decode_packet_group_medium {
    use crate::test_cases::*;
    use divan::{black_box, Bencher};

    static CASE: &Case = &ALL_CASES[2];

    bench_group_decode_packet!(treenode_decode_packet_traverse_baseline);
    bench_group_decode_packet!(treenode_decode_packet_traverse);
    bench_group_decode_packet!(flatnode_decode_packet_traverse);
}

#[divan::bench_group(sample_count = 100_000)]
mod decode_packet_group_medium_small {
    use crate::test_cases::*;
    use divan::{black_box, Bencher};

    static CASE: &Case = &ALL_CASES[3];

    bench_group_decode_packet!(treenode_decode_packet_traverse_baseline);
    bench_group_decode_packet!(treenode_decode_packet_traverse);
    bench_group_decode_packet!(flatnode_decode_packet_traverse);
}

#[divan::bench_group(sample_count = 100_000)]
mod decode_packet_group_small {
    use crate::test_cases::*;
    use divan::{black_box, Bencher};

    static CASE: &Case = &ALL_CASES[4];

    bench_group_decode_packet!(treenode_decode_packet_traverse_baseline);
    bench_group_decode_packet!(treenode_decode_packet_traverse);
    bench_group_decode_packet!(flatnode_decode_packet_traverse);
}

#[divan::bench_group(sample_count = 100_000)]
mod decode_packet_group_small_min {
    use crate::test_cases::*;
    use divan::{black_box, Bencher};

    static CASE: &Case = &ALL_CASES[5];

    bench_group_decode_packet!(treenode_decode_packet_traverse_baseline);
    bench_group_decode_packet!(treenode_decode_packet_traverse);
    bench_group_decode_packet!(flatnode_decode_packet_traverse);
}
