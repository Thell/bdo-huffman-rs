use bit_vec::BitVec;

use crate::{node::TreeNode, packet::Packet};

// Nested TreeNode Traversal - Original
pub fn decode_packet_nested_orig(content: &[u8]) -> String {
    let packet = Packet::new(content);
    let frequencies = packet.symbol_table();
    let tree = packet.nested_tree(&frequencies);

    let mut encoded_bits = BitVec::from_bytes(&packet.encoded_message);
    encoded_bits.truncate(packet.bitstream_len as usize);

    decode_message_nested_orig(&encoded_bits, packet.decoded_bytes_len, &tree)
}

// Nested TreeNode Traversal - Original
pub fn decode_message_nested_orig(
    encoded: &BitVec,
    decoded_len: u32,
    nested_tree: &TreeNode,
) -> String {
    let mut result = String::with_capacity(decoded_len as usize);
    let mut current = nested_tree;

    for bit in encoded.iter() {
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
            current = nested_tree;
        }
    }
    result
}

// MARK: Unit Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_cases::*;

    #[test]
    fn decodes_message_nested_orig() {
        let packet = Packet::new(&TEST_BYTES);
        let nested_tree = packet.nested_tree(&EXPECTED_SYMBOL_TABLE);

        let mut encoded_bits = BitVec::from_bytes(&packet.encoded_message);
        encoded_bits.truncate(packet.bitstream_len as usize);

        let decoded_message =
            decode_message_nested_orig(&encoded_bits, packet.decoded_bytes_len, &nested_tree);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn processes_packet_nested_orig() {
        let decoded_message = decode_packet_nested_orig(&TEST_BYTES);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }
}

// MARK: Benches

#[divan::bench_group(sample_count = 10_000)]
mod benches {
    use super::*;
    use crate::test_cases::*;

    use divan::counter::{BytesCount, ItemsCount};
    use divan::{black_box, Bencher};

    #[divan::bench(args = ALL_CASES)]
    fn packet_decoding_nested_orig(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();

        bencher
            .counter(ItemsCount::from(1usize))
            .bench_local(move || {
                decode_packet_nested_orig(black_box(&response_bytes));
            });
    }

    #[divan::bench(args = ALL_CASES)]
    fn message_decoding_nested_orig(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = Packet::new(response_bytes);
        let symbol_table = &packet.symbol_table();
        let nested_tree = &packet.nested_tree(symbol_table);

        let mut encoded_bits = BitVec::from_bytes(&packet.encoded_message);
        encoded_bits.truncate(packet.bitstream_len as usize);

        bencher
            .counter(BytesCount::from(packet.decoded_bytes_len))
            .bench_local(move || {
                decode_message_nested_orig(
                    black_box(&encoded_bits),
                    packet.decoded_bytes_len,
                    &nested_tree,
                );
            });
    }
}
