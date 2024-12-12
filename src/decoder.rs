use bit_vec::BitVec;

use crate::{node::TreeNode, packet::Packet};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Nested TreeNode Traversal - Baseline
pub fn decode_packet_nested_baseline(content: &[u8]) -> String {
    let packet = Packet::new(content);
    let frequencies = packet.symbol_table();
    let tree = packet.nested_tree(&frequencies);

    let mut encoded_bits = BitVec::from_bytes(&packet.encoded_message);
    encoded_bits.truncate(packet.bitstream_len as usize);

    decode_message_nested_baseline(&encoded_bits, packet.decoded_bytes_len, &tree)
}

pub fn decode_message_nested_baseline(
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

////////////////////////////////////////////////////////////////////////////////////////////////////
// Nested TreeNode Traversal - Optimized
pub fn decode_packet_nested_optimized(content: &[u8]) -> String {
    let packet = Packet::new(content);
    let symbol_table = packet.symbol_table();
    let tree = packet.nested_tree(&symbol_table);

    decode_message_nested_optimized(&packet, &tree)
}

// This isn't good for an optimized decoder but with no setup and short messages (<75k) it is 'ok'.
// The time compared to baseline is halved across all test sizes.
pub fn decode_message_nested_optimized(packet: &Packet, nested_tree: &TreeNode) -> String {
    // SAFETY: No guarantees of safety here as the assumptions are that:
    // - packet is valid and well formed
    // - nested_tree is valid and corresponds to packet data

    // Construct the Vec<u8> using a mut_ptr and zero-copy convert to a string when done.
    let mut decoded: Vec<u8> = Vec::with_capacity(packet.decoded_bytes_len as usize);
    let mut current = nested_tree;
    let mut read_index = 0;
    let mut write_index = 0;

    // Remove the bitvec iteration and process one byte at a time.
    // The read turns into a single movzx of a byte_ptr.
    'outer: loop {
        let mut bits = unsafe { *packet.encoded_message.get_unchecked(read_index) };
        read_index += 1;

        // Compiler unrolls the loop and optimizes down to:
        // -  7 instructions and  6 uops per bit iteration without a symbol.
        // - 12 instructions and 14 uops per bit iteration when a symbol is found.
        for _ in 0..8 {
            let bit = (bits & 0b1000_0000) != 0;
            bits <<= 1;

            unsafe {
                current = if bit {
                    current.right_child.as_deref().unwrap_unchecked()
                } else {
                    current.left_child.as_deref().unwrap_unchecked()
                };
            }

            if let Some(symbol) = current.symbol {
                unsafe {
                    *decoded.as_mut_ptr().add(write_index) = symbol;
                }
                write_index += 1;
                current = nested_tree;

                if write_index == packet.decoded_bytes_len as usize {
                    break 'outer;
                }
            }
        }
    }

    unsafe {
        decoded.set_len(write_index);

        // Zero-copy conversion of the decoded buffer to a String.
        let slice = std::slice::from_raw_parts(decoded.as_ptr(), decoded.len());
        std::str::from_utf8_unchecked(slice).to_owned()
    }
}

// MARK: Unit Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_cases::*;

    #[test]
    fn processes_packet_nested_baseline() {
        let decoded_message = decode_packet_nested_baseline(&TEST_BYTES);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn decodes_message_nested_baseline() {
        let packet = Packet::new(&TEST_BYTES);
        let nested_tree = packet.nested_tree(&EXPECTED_SYMBOL_TABLE);

        let mut encoded_bits = BitVec::from_bytes(&packet.encoded_message);
        encoded_bits.truncate(packet.bitstream_len as usize);

        let decoded_message =
            decode_message_nested_baseline(&encoded_bits, packet.decoded_bytes_len, &nested_tree);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn processes_packet_nested_optimized() {
        let decoded_message = decode_packet_nested_optimized(&TEST_BYTES);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn decodes_message_nested_optimized() {
        let packet = Packet::new(&TEST_BYTES);
        let nested_tree = packet.nested_tree(&EXPECTED_SYMBOL_TABLE);
        let decoded_message = decode_message_nested_optimized(&packet, &nested_tree);

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
    fn packet_decoding_nested_baseline(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();

        bencher
            .counter(ItemsCount::from(1usize))
            .bench_local(move || {
                decode_packet_nested_baseline(black_box(&response_bytes));
            });
    }

    #[divan::bench(args = ALL_CASES)]
    fn message_decoding_nested_baseline(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = Packet::new(response_bytes);
        let symbol_table = &packet.symbol_table();
        let nested_tree = &packet.nested_tree(symbol_table);

        let mut encoded_bits = BitVec::from_bytes(&packet.encoded_message);
        encoded_bits.truncate(packet.bitstream_len as usize);

        bencher
            .counter(BytesCount::from(packet.decoded_bytes_len))
            .bench_local(move || {
                decode_message_nested_baseline(
                    black_box(&encoded_bits),
                    packet.decoded_bytes_len,
                    &nested_tree,
                );
            });
    }

    #[divan::bench(args = ALL_CASES)]
    fn packet_decoding_nested_optimized(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();

        bencher
            .counter(ItemsCount::from(1usize))
            .bench_local(move || {
                decode_packet_nested_optimized(black_box(&response_bytes));
            });
    }

    #[divan::bench(args = ALL_CASES)]
    fn message_decoding_nested_optimized(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = &Packet::new(&response_bytes);
        let symbol_table = &packet.symbol_table();
        let nested_tree = packet.nested_tree(symbol_table);

        bencher
            .counter(BytesCount::from(packet.decoded_bytes_len))
            .bench_local(move || {
                decode_message_nested_optimized(black_box(&packet), &nested_tree);
            });
    }
}
