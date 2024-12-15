use bit_vec::BitVec;

use crate::{
    node::TreeNode,
    packet::{ExtendedPrefix, Packet},
};

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

////////////////////////////////////////////////////////////////////////////////////////////////////
// Flat Node Traversal
pub fn decode_packet_with_table(content: &[u8]) -> String {
    let packet = Packet::new(content);
    let symbol_table = packet.symbol_table();
    let tree = packet.flat_tree(&symbol_table);
    let prefixes = packet.extended_prefixes(&tree);
    decode_message_with_table(&packet, &prefixes)
}

// This approach uses a table of entries that contain symbols and bits used.
// Each integer 0..=255 is decoded using tree traversal to build the table.
// You are encouraged to profile this approach as well as view it in Godbolt as you can see the
// hit counts at each of the symbol length checks in the profiler and see that there are
// 8 instructions for each symbol past the first of each index using 9 uops.
// This function really seems to benefit from look-ahead processing.
pub fn decode_message_with_table(packet: &Packet, table: &Vec<ExtendedPrefix>) -> String {
    let decoded_len = packet.decoded_bytes_len as usize;
    let mut decoded: Vec<u8> = Vec::with_capacity(decoded_len);

    let mut bit_buf = 0u16;
    let mut bit_buf_remaining = 0;
    let mut read_index = 0usize;
    let mut write_index = 0usize;
    let mut prefix_used_bits = 0;

    loop {
        bit_buf_remaining -= prefix_used_bits;

        if bit_buf_remaining < 8 && read_index < packet.encoded_message.len() {
            let incoming_bits = packet.encoded_message[read_index] as u16;
            bit_buf |= incoming_bits << (8 - bit_buf_remaining);
            bit_buf_remaining += 8;
            read_index += 1;
        }

        let index = (bit_buf >> 8) as usize;

        unsafe {
            let extended_prefix = table.get_unchecked(index);
            let symbols = &extended_prefix.symbols;
            prefix_used_bits = extended_prefix.used_bits;

            bit_buf <<= extended_prefix.used_bits;

            *decoded.as_mut_ptr().add(write_index) = *symbols.get_unchecked(0);
            write_index += 1;
            if write_index == decoded_len {
                break;
            }

            if symbols.len() > 1 {
                *decoded.as_mut_ptr().add(write_index) = *symbols.get_unchecked(1);
                write_index += 1;
                if write_index == decoded_len {
                    break;
                }
            }
            if symbols.len() > 2 {
                *decoded.as_mut_ptr().add(write_index) = *symbols.get_unchecked(2);
                write_index += 1;
                if write_index == decoded_len {
                    break;
                }
            }
            if symbols.len() > 3 {
                *decoded.as_mut_ptr().add(write_index) = *symbols.get_unchecked(3);
                write_index += 1;
                if write_index == decoded_len {
                    break;
                }
            }
            if symbols.len() > 4 {
                *decoded.as_mut_ptr().add(write_index) = *symbols.get_unchecked(4);
                write_index += 1;
                if write_index == decoded_len {
                    break;
                }
            }
            if symbols.len() > 5 {
                *decoded.as_mut_ptr().add(write_index) = *symbols.get_unchecked(5);
                write_index += 1;
                if write_index == decoded_len {
                    break;
                }
            }
            if symbols.len() > 6 {
                *decoded.as_mut_ptr().add(write_index) = *symbols.get_unchecked(6);
                write_index += 1;
                if write_index == decoded_len {
                    break;
                }
            }
            if symbols.len() > 7 {
                *decoded.as_mut_ptr().add(write_index) = *symbols.get_unchecked(7);
                write_index += 1;
                if write_index == decoded_len {
                    break;
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
// SIMD Table Flat Node Traversal
pub fn decode_packet_simd_table(content: &[u8]) -> String {
    let packet = Packet::new(content);
    let symbol_table = packet.symbol_table();
    let tree = packet.flat_tree(&symbol_table);
    let prefixes = packet.extended_prefixes(&tree);
    decode_message_simd_table(&packet, &prefixes)
}

pub fn decode_message_simd_table(packet: &Packet, table: &Vec<ExtendedPrefix>) -> String {
    // let mut result = String::new();
    let decoded_len = packet.decoded_bytes_len as usize;
    let mut decoded: Vec<u8> = Vec::with_capacity(decoded_len);

    let mut write_index = 0;
    let encoded_bits = &packet.encoded_message;

    let mut bit_pos = 0;

    'outer: while bit_pos < encoded_bits.len() * 8 {
        // Prepare SIMD bits
        let mut simd_data = [0u8; 32];
        for i in 0..32 {
            let byte_index = (bit_pos + i) / 8;
            let bit_offset = (bit_pos + i) % 8;

            if byte_index < encoded_bits.len() {
                let mut value = unsafe { encoded_bits.get_unchecked(byte_index) << bit_offset };

                // Only add bits from the next byte if `bit_offset > 0`
                if bit_offset > 0 && byte_index + 1 < encoded_bits.len() {
                    value |= unsafe {
                        ((*encoded_bits.get_unchecked(byte_index + 1) as u16) >> (8 - bit_offset))
                            as u8
                    };
                }

                unsafe { *simd_data.get_unchecked_mut(i) = (value & 0xFF) as u8 };
            // Keep only the lower 8 bits
            } else {
                unsafe { *simd_data.get_unchecked_mut(i) = 0 }; // Zero-fill for out-of-bounds
            }
        }

        // Load the data into an SIMD register using intrinsics
        let simd_vec =
            unsafe { std::arch::x86_64::_mm256_loadu_si256(simd_data.as_ptr() as *const _) };

        // Extract the indices from the SIMD register
        let mut indices = [0u8; 32];
        unsafe { std::arch::x86_64::_mm256_storeu_si256(indices.as_mut_ptr() as *mut _, simd_vec) };

        // Perform table lookups and result writes
        let mut local_bit_pos = 0;
        let mut current_lane = 0;
        while current_lane < 32 {
            let index = unsafe { *indices.get_unchecked(current_lane as usize) as usize };
            let extended_prefix = unsafe { table.get_unchecked(index) };
            let symbols = &extended_prefix.symbols;

            unsafe {
                // result.extend_from_slice(&symbols);
                *decoded.as_mut_ptr().add(write_index) = *symbols.get_unchecked(0);
                write_index += 1;
                if write_index == decoded_len {
                    break 'outer;
                }

                if symbols.len() > 1 {
                    *decoded.as_mut_ptr().add(write_index) = *symbols.get_unchecked(1);
                    write_index += 1;
                    if write_index == decoded_len {
                        break 'outer;
                    }
                }
                if symbols.len() > 2 {
                    *decoded.as_mut_ptr().add(write_index) = *symbols.get_unchecked(2);
                    write_index += 1;
                    if write_index == decoded_len {
                        break 'outer;
                    }
                }
                if symbols.len() > 3 {
                    *decoded.as_mut_ptr().add(write_index) = *symbols.get_unchecked(3);
                    write_index += 1;
                    if write_index == decoded_len {
                        break 'outer;
                    }
                }
                if symbols.len() > 4 {
                    *decoded.as_mut_ptr().add(write_index) = *symbols.get_unchecked(4);
                    write_index += 1;
                    if write_index == decoded_len {
                        break 'outer;
                    }
                }
                if symbols.len() > 5 {
                    *decoded.as_mut_ptr().add(write_index) = *symbols.get_unchecked(5);
                    write_index += 1;
                    if write_index == decoded_len {
                        break 'outer;
                    }
                }
                if symbols.len() > 6 {
                    *decoded.as_mut_ptr().add(write_index) = *symbols.get_unchecked(6);
                    write_index += 1;
                    if write_index == decoded_len {
                        break 'outer;
                    }
                }
                if symbols.len() > 7 {
                    *decoded.as_mut_ptr().add(write_index) = *symbols.get_unchecked(7);
                    write_index += 1;
                    if write_index == decoded_len {
                        break 'outer;
                    }
                }
            }

            let bits_to_advance = extended_prefix.used_bits as usize;
            current_lane += bits_to_advance;
            local_bit_pos += bits_to_advance;

            if current_lane >= 32 {
                break;
            }
        }
        bit_pos += local_bit_pos;
    }

    unsafe {
        decoded.set_len(write_index);
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
        // Tests the integrity of the full processing flow.
        let decoded_message = decode_packet_nested_baseline(&TEST_BYTES);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn decodes_message_nested_baseline() {
        // Tests only the decoding algo.
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
        // Tests the integrity of the full processing flow.
        let decoded_message = decode_packet_nested_optimized(&TEST_BYTES);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn decodes_message_nested_optimized() {
        // Tests only the decoding algo.
        let packet = Packet::new(&TEST_BYTES);
        let nested_tree = packet.nested_tree(&EXPECTED_SYMBOL_TABLE);
        let decoded_message = decode_message_nested_optimized(&packet, &nested_tree);

        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn all_samples_baseline_vs_optimized_baseline() {
        for case in SAMPLE_CASES {
            println!("case: {}_{}", case.main_category, case.sub_category);
            let content = &case.request();
            let base_result = decode_packet_nested_baseline(&content);
            let optimized_result = decode_packet_nested_optimized(&content);
            assert_eq!(base_result, optimized_result)
        }
    }

    #[test]
    fn processes_packet_with_table() {
        // Tests the integrity of the full processing flow.
        let decoded_message = decode_packet_with_table(&TEST_BYTES);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn decodes_message_with_table() {
        // Tests only the decoding algo.
        let packet = Packet::new(&TEST_BYTES);
        let tree = &packet.flat_tree(&EXPECTED_SYMBOL_TABLE);
        let prefixes = packet.prefixes_from_flatnode(tree);
        for prefix in prefixes {
            println!("{:?}", prefix);
        }
        let prefix_table = packet.extended_prefixes(tree);
        println!("{:?}", prefix_table);
        let decoded_message = decode_message_with_table(&packet, &prefix_table);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn all_samples_baseline_vs_table() {
        for case in SAMPLE_CASES {
            println!("case: {}_{}", case.main_category, case.sub_category);
            let content = &case.request();
            let base_result = decode_packet_nested_baseline(&content);
            let table_result = decode_packet_with_table(&content);
            assert_eq!(base_result, table_result)
        }
    }

    #[test]
    fn processes_packet_with_table_simd() {
        // Tests the integrity of the full processing flow.
        let decoded_message = decode_packet_simd_table(&TEST_BYTES);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn decodes_message_with_table_simd() {
        // Tests only the decoding algo.
        let packet = Packet::new(&TEST_BYTES);
        let tree = &packet.flat_tree(&EXPECTED_SYMBOL_TABLE);
        let prefixes = packet.prefixes_from_flatnode(tree);
        for prefix in prefixes {
            println!("{:?}", prefix);
        }
        let prefix_table = packet.extended_prefixes(tree);
        let decoded_message = decode_message_simd_table(&packet, &prefix_table);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn all_samples_baseline_vs_simd() {
        for case in SAMPLE_CASES {
            println!("case: {}_{}", case.main_category, case.sub_category);
            let content = &case.request();
            let base_result = decode_packet_nested_baseline(&content);
            let simd_result = decode_packet_simd_table(&content);
            assert_eq!(base_result, simd_result)
        }
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

    #[divan::bench(args = ALL_CASES)]
    fn packet_decoding_with_table(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();

        bencher
            .counter(ItemsCount::from(1usize))
            .bench_local(move || {
                black_box(decode_packet_with_table(black_box(response_bytes)));
            });
    }

    #[divan::bench(args = ALL_CASES)]
    fn message_decoding_with_table(bencher: Bencher, case: &Case) {
        // Tests only the decoding algo.
        let response_bytes = &case.request();
        let packet = Packet::new(response_bytes);
        let tree = &packet.flat_tree(&packet.symbol_table());
        let prefix_table = &packet.extended_prefixes(tree);

        bencher
            .counter(BytesCount::from(packet.decoded_bytes_len))
            .bench_local(move || {
                decode_message_with_table(black_box(&packet), prefix_table);
            });
    }

    #[divan::bench(args = ALL_CASES)]
    fn packet_decoding_with_table_simd(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();

        bencher
            .counter(ItemsCount::from(1usize))
            .bench_local(move || {
                black_box(decode_packet_simd_table(black_box(response_bytes)));
            });
    }

    #[divan::bench(args = ALL_CASES)]
    fn message_decoding_with_table_simd(bencher: Bencher, case: &Case) {
        // Tests only the decoding algo.
        let response_bytes = &case.request();
        let packet = Packet::new(response_bytes);
        let tree = &packet.flat_tree(&packet.symbol_table());
        let prefix_table = &packet.extended_prefixes(tree);

        bencher
            .counter(BytesCount::from(packet.decoded_bytes_len))
            .bench_local(move || {
                decode_message_simd_table(black_box(&packet), prefix_table);
            });
    }
}
