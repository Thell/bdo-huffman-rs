use crate::node::{FlatNode, FlatNodeSafe};
use crate::{node::TreeNode, packet::Packet};

use bit_vec::BitVec;
use bitter::{BigEndianReader, BitReader};

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
// Tree Traversal - TreeNode - A fully safe version
pub fn treenode_decode_packet_traverse_safe(content: &[u8]) -> String {
    let packet = Packet::new(content);
    let tree = packet.treenode_tree();
    treenode_decode_message_traverse_safe(&packet, &tree)
}

pub fn treenode_decode_message_traverse_safe(packet: &Packet, tree: &TreeNode) -> String {
    let mut decoded: Vec<u8> = vec![0; packet.decoded_bytes_len as usize];
    let mut current = tree;
    let mut write_index = 0;

    'outer: for byte in packet.encoded_message.iter() {
        let mut bits = *byte;
        for _ in 0..8 {
            let bit = (bits & 0b1000_0000) != 0;
            bits <<= 1;

            let child = match bit {
                true => current.right_child.as_ref().unwrap(),
                false => current.left_child.as_ref().unwrap(),
            };

            current = child;

            if let Some(symbol) = current.symbol {
                decoded[write_index] = symbol;
                write_index += 1;
                current = tree;

                if write_index == packet.decoded_bytes_len as usize {
                    break 'outer;
                }
            }
        }
    }

    let slice = &decoded[..];
    std::str::from_utf8(slice).unwrap().to_owned()
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Tree Traversal - TreeNode - A fully unsafe version
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
// Tree Traversal - FlatNode - A fully safe version
pub fn flatnode_decode_packet_traverse_safe_index(content: &[u8]) -> String {
    let packet = Packet::new(content);

    // Safe version
    let tree = packet.flatnode_tree_safe();
    flatnode_decode_message_traverse_safe_index(&packet, &tree)
}

pub fn flatnode_decode_message_traverse_safe_index(
    packet: &Packet,
    tree: &[FlatNodeSafe],
) -> String {
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

////////////////////////////////////////////////////////////////////////////////////////////////////
// Tree Traversal - FlatNode - A minimally unsafe version
pub fn flatnode_decode_packet_traverse_safe_const(content: &[u8]) -> String {
    let packet = &Packet::new(content);
    let tree = packet.flatnode_tree();
    flatnode_decode_message_traverse_safe_const(packet, &tree)
}

pub fn flatnode_decode_message_traverse_safe_const(packet: &Packet, tree: &[FlatNode]) -> String {
    let mut decoded: Vec<u8> = vec![0; packet.decoded_bytes_len as usize];
    let root = &tree[0];
    let mut node = root;
    let mut write_index = 0;

    'outer: for byte in packet.encoded_message.iter() {
        let mut bits = *byte;

        for _ in 0..8 {
            let direction = (bits & 0b1000_0000) != 0;

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
            bits <<= 1;
        }
    }

    let slice = &decoded[..];
    std::str::from_utf8(slice).unwrap().to_owned()
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Tree Traversal - FlatNode
pub fn flatnode_decode_packet_traverse(content: &[u8]) -> String {
    let packet = &Packet::new(content);
    let tree = packet.flatnode_tree();
    flatnode_decode_message_traverse(packet, &tree)
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
            if let Some(symbol) = node.symbol {
                unsafe {
                    *decoded.as_mut_ptr().add(write_index) = symbol;
                }
                write_index += 1;
                if write_index == packet.decoded_bytes_len as usize {
                    break 'outer;
                }
                node = root;
            }
            bits <<= 1;
        }
    }

    unsafe {
        decoded.set_len(write_index);
        let slice = std::slice::from_raw_parts(decoded.as_ptr(), decoded.len());
        std::str::from_utf8_unchecked(slice).to_owned()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Table Lookup - PrefixTable - A fully safe version
pub fn flatnode_decode_packet_prefix_table_safe_index(content: &[u8]) -> String {
    let packet = &Packet::new(content);
    let flat_tree = &packet.flatnode_tree_safe();
    let extended_prefix_entries = &packet.flatnode_prefix_table_safe_index(flat_tree);
    flatnode_decode_message_prefix_table_safe(packet, extended_prefix_entries)
}

// Table Lookup - PrefixTable - A minimally unsafe version
pub fn flatnode_decode_packet_prefix_table_safe_const(content: &[u8]) -> String {
    let packet = &Packet::new(content);
    let flat_tree = &packet.flatnode_tree();
    let extended_prefix_entries = &packet.flatnode_prefix_table_safe_const(flat_tree);
    flatnode_decode_message_prefix_table_safe(packet, extended_prefix_entries)
}

pub fn flatnode_decode_message_prefix_table_safe(
    packet: &Packet,
    table: &crate::packet::PrefixTableSafe,
) -> String {
    let mut decoded: Vec<u8> = vec![0; packet.decoded_bytes_len as usize + 8];
    let mut write_index = 0usize;

    let mut bits = BigEndianReader::new(packet.encoded_message);

    // Lookahead is 56bits
    // Consume unbuffered bytes by processing 7 8-bit indices per iteration.
    // This does not consume all bits in lookahead on each iteration.
    while bits.unbuffered_bytes_remaining() > 7 {
        bits.refill_lookahead();
        lookup_unchecked_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
        lookup_unchecked_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
        lookup_unchecked_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
        lookup_unchecked_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
        lookup_unchecked_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
        lookup_unchecked_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
        lookup_unchecked_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
    }

    // Drain unbuffered bytes with safe refill.
    while bits.unbuffered_bytes_remaining() > 0 {
        bits.refill_lookahead();
        lookup_unchecked_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
    }

    // Consume lookahead without refill or peek checks until the last byte.
    while bits.has_bits_remaining(8) {
        lookup_unchecked_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
    }

    // Drain partial byte bits with peek checks.
    while bits.has_bits_remaining(1) && write_index < packet.decoded_bytes_len as usize {
        lookup_prefix_table_safe(&mut bits, table, &mut write_index, &mut decoded);
    }

    // Truncate decoded since write_index wasn't while writing symbols.
    let slice = &decoded[..];
    let mut decoded = std::str::from_utf8(slice).unwrap().to_owned();
    decoded.truncate(packet.decoded_bytes_len as usize);
    decoded
}

#[inline(always)]
fn lookup_unchecked_prefix_table_safe(
    bits: &mut BigEndianReader,
    table: &crate::packet::PrefixTableSafe,
    write_index: &mut usize,
    decoded: &mut [u8],
) {
    let index = bits.peek(8) as usize;

    let symbols = &table.symbols[index];
    let len = table.lens[index] as usize;
    let used_bits = table.bits_used[index];

    get_symbols_unchecked_safe(symbols, len, write_index, decoded);
    bits.consume(used_bits as u32);
}

#[inline(always)]
fn lookup_prefix_table_safe(
    bits: &mut BigEndianReader,
    table: &crate::packet::PrefixTableSafe,
    write_index: &mut usize,
    decoded: &mut [u8],
) {
    let lookahead_count = bits.lookahead_bits().min(8);
    let last_bits = bits.peek(lookahead_count);
    let index = (last_bits << (8 - lookahead_count)) as usize;

    let symbols = &table.symbols[index];
    let len = table.lens[index] as usize;
    let used_bits = table.bits_used[index];

    get_symbols_unchecked_safe(symbols, len, write_index, decoded);

    let bits_to_consume = lookahead_count.min(used_bits as u32);
    bits.consume(bits_to_consume);
}

#[inline(always)]
fn get_symbols_unchecked_safe(
    symbols: &[u8],
    len: usize,
    write_index: &mut usize,
    decoded: &mut [u8],
) {
    // Manually unroll the loop for performance. Approximately 20% speedup.
    decoded[*write_index] = symbols[0];
    *write_index += 1;

    if len > 1 {
        decoded[*write_index] = symbols[1];
        *write_index += 1;
    } else {
        return;
    }
    if len > 2 {
        decoded[*write_index] = symbols[2];
        *write_index += 1;
    } else {
        return;
    }
    if len > 3 {
        decoded[*write_index] = symbols[3];
        *write_index += 1;
    } else {
        return;
    }
    if len > 4 {
        decoded[*write_index] = symbols[4];
        *write_index += 1;
    } else {
        return;
    }
    if len > 5 {
        decoded[*write_index] = symbols[5];
        *write_index += 1;
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Table Lookup - PrefixTable - A fully unsafe version using const ptr.
pub fn flatnode_decode_packet_prefix_table(content: &[u8]) -> String {
    let packet = &Packet::new(content);
    let flat_tree = &packet.flatnode_tree();
    let extended_prefix_entries = &packet.flatnode_prefix_table(flat_tree);
    flatnode_decode_message_prefix_table(packet, extended_prefix_entries)
}

pub fn flatnode_decode_message_prefix_table(
    packet: &Packet,
    table: &crate::packet::PrefixTable,
) -> String {
    let decoded_len = packet.decoded_bytes_len as usize;
    let mut decoded: Vec<u8> = Vec::with_capacity(decoded_len + 8);
    let mut write_index = 0usize;

    let mut bits = BigEndianReader::new(packet.encoded_message);

    // Lookahead is 56bits
    // Consume unbuffered bytes by processing 7 8-bit indices per iteration.
    // This does not consume all bits in lookahead on each iteration.
    while bits.unbuffered_bytes_remaining() > 7 {
        // Manually unroll the loop for performance. Approximately 12% speedup.
        unsafe {
            bits.refill_lookahead_unchecked();
            lookup_unchecked_prefix_table(&mut bits, table, &mut write_index, &mut decoded);
            lookup_unchecked_prefix_table(&mut bits, table, &mut write_index, &mut decoded);
            lookup_unchecked_prefix_table(&mut bits, table, &mut write_index, &mut decoded);
            lookup_unchecked_prefix_table(&mut bits, table, &mut write_index, &mut decoded);
            lookup_unchecked_prefix_table(&mut bits, table, &mut write_index, &mut decoded);
            lookup_unchecked_prefix_table(&mut bits, table, &mut write_index, &mut decoded);
            lookup_unchecked_prefix_table(&mut bits, table, &mut write_index, &mut decoded);
        }
    }

    // Drain unbuffered bytes with safe refill.
    while bits.unbuffered_bytes_remaining() > 0 {
        bits.refill_lookahead();
        unsafe { lookup_unchecked_prefix_table(&mut bits, table, &mut write_index, &mut decoded) };
    }

    // Consume lookahead without refill or peek checks until the last byte.
    while bits.has_bits_remaining(8) {
        unsafe { lookup_unchecked_prefix_table(&mut bits, table, &mut write_index, &mut decoded) }
    }

    // Drain partial byte bits with peek checks.
    while bits.has_bits_remaining(1) {
        unsafe { lookup_prefix_table(&mut bits, table, &mut write_index, &mut decoded) }
    }

    // Truncate decoded since write_index wasn't while draining lookahead.
    unsafe {
        decoded.set_len(write_index);
        let slice = std::slice::from_raw_parts(decoded.as_ptr(), decoded.len());
        let mut decoded = std::str::from_utf8_unchecked(slice).to_owned();
        decoded.truncate(decoded_len);
        decoded
    }
}

#[inline(always)]
unsafe fn lookup_unchecked_prefix_table(
    bits: &mut BigEndianReader,
    table: &crate::packet::PrefixTable,
    write_index: &mut usize,
    decoded: &mut [u8],
) {
    let index = bits.peek(8) as usize;

    let symbols = table.symbols.get_unchecked(index);
    let used_bits = *table.bits_used.get_unchecked(index);

    get_symbols_unchecked(symbols, write_index, decoded);
    bits.consume(used_bits as u32);
}

#[inline(always)]
unsafe fn lookup_prefix_table(
    bits: &mut BigEndianReader,
    table: &crate::packet::PrefixTable,
    write_index: &mut usize,
    decoded: &mut [u8],
) {
    let lookahead_count = bits.lookahead_bits().min(8);
    let last_bits = bits.peek(lookahead_count);
    let index = (last_bits << (8 - lookahead_count)) as usize;

    let symbols = table.symbols.get_unchecked(index);
    let used_bits = *table.bits_used.get_unchecked(index);

    get_symbols_unchecked(symbols, write_index, decoded);

    let bits_to_consume = lookahead_count.min(used_bits as u32);
    bits.consume(bits_to_consume);
}

#[inline(always)]
unsafe fn get_symbols_unchecked(symbols: &[u8], write_index: &mut usize, decoded: &mut [u8]) {
    // Manually unroll the loop for performance. Approximately 50% speedup.
    *decoded.as_mut_ptr().add(*write_index) = *symbols.get_unchecked(0);
    *write_index += 1;

    if *symbols.get_unchecked(1) > 0 {
        *decoded.as_mut_ptr().add(*write_index) = *symbols.get_unchecked(1);
        *write_index += 1;
    } else {
        return;
    }
    if *symbols.get_unchecked(2) > 0 {
        *decoded.as_mut_ptr().add(*write_index) = *symbols.get_unchecked(2);
        *write_index += 1;
    } else {
        return;
    }
    if *symbols.get_unchecked(3) > 0 {
        *decoded.as_mut_ptr().add(*write_index) = *symbols.get_unchecked(3);
        *write_index += 1;
    } else {
        return;
    }
    if *symbols.get_unchecked(4) > 0 {
        *decoded.as_mut_ptr().add(*write_index) = *symbols.get_unchecked(4);
        *write_index += 1;
    } else {
        return;
    }
    if *symbols.get_unchecked(5) > 0 {
        *decoded.as_mut_ptr().add(*write_index) = *symbols.get_unchecked(5);
        *write_index += 1;
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
    fn treenode_decode_packet_traverse_safe() {
        let decoded_message = super::treenode_decode_packet_traverse_safe(&TEST_BYTES);
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
    fn flatnode_decode_packet_traverse_safe_rc() {
        let decoded_message = super::flatnode_decode_packet_traverse_safe_index(&TEST_BYTES);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn flatnode_decode_message_traverse_safe_index() {
        let packet = Packet::new(&TEST_BYTES);
        let tree = packet.flatnode_tree_safe();
        let decoded_message = super::flatnode_decode_message_traverse_safe_index(&packet, &tree);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn all_samples_baseline_vs_flatnode_traverse_safe_index() {
        for case in SAMPLE_CASES {
            println!("case: {}_{}", case.main_category, case.sub_category);
            let content = &case.request();
            let expected_result = super::treenode_decode_packet_traverse_baseline(&content);
            let result = super::flatnode_decode_packet_traverse_safe_index(&content);
            assert_eq!(result, expected_result)
        }
    }

    #[test]
    fn flatnode_decode_packet_traverse_safe_const() {
        let decoded_message = super::flatnode_decode_packet_traverse_safe_const(&TEST_BYTES);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn flatnode_decode_message_traverse_safe_const() {
        let packet = &Packet::new(&TEST_BYTES);
        let tree = packet.flatnode_tree();
        let decoded_message = super::flatnode_decode_message_traverse_safe_const(&packet, &tree);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn all_samples_baseline_vs_flatnode_traverse_safe_const() {
        for case in SAMPLE_CASES {
            println!("case: {}_{}", case.main_category, case.sub_category);
            let content = &case.request();
            let expected_result = super::treenode_decode_packet_traverse_baseline(&content);
            let result = super::flatnode_decode_packet_traverse_safe_const(&content);
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
        let packet = &Packet::new(&TEST_BYTES);
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

    #[test]
    fn flatnode_decode_packet_prefix_table_safe_index() {
        let decoded_message = super::flatnode_decode_packet_prefix_table_safe_index(&TEST_BYTES);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn flatnode_decode_message_prefix_table_safe_index() {
        let packet = Packet::new(&TEST_BYTES);
        let tree = packet.flatnode_tree_safe();
        let table = packet.flatnode_prefix_table_safe_index(&tree);
        let decoded_message = super::flatnode_decode_message_prefix_table_safe(&packet, &table);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn all_samples_baseline_to_prefix_table_safe_index() {
        for case in SAMPLE_CASES.iter().rev() {
            println!("case: {}_{}", case.main_category, case.sub_category);
            let content = case.request();
            let expected_result = super::treenode_decode_packet_traverse_safe(&content);
            let result = super::flatnode_decode_packet_prefix_table_safe_index(&content);
            assert_eq!(result, expected_result)
        }
    }

    #[test]
    fn flatnode_decode_packet_prefix_table_safe_const() {
        let decoded_message = super::flatnode_decode_packet_prefix_table_safe_const(&TEST_BYTES);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn flatnode_decode_message_prefix_table_safe_const() {
        let packet = &Packet::new(&TEST_BYTES);
        let tree = packet.flatnode_tree();
        let table = packet.flatnode_prefix_table_safe_const(&tree);
        let decoded_message = super::flatnode_decode_message_prefix_table_safe(&packet, &table);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn all_samples_baseline_to_prefix_table_safe_const() {
        for case in SAMPLE_CASES.iter().rev() {
            println!("case: {}_{}", case.main_category, case.sub_category);
            let content = case.request();
            let expected_result = super::treenode_decode_packet_traverse_safe(&content);
            let result = super::flatnode_decode_packet_prefix_table_safe_const(&content);
            assert_eq!(result, expected_result)
        }
    }

    #[test]
    fn flatnode_decode_packet_prefix_table() {
        let decoded_message = super::flatnode_decode_packet_prefix_table(&TEST_BYTES);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn flatnode_decode_message_prefix_table() {
        let packet = &Packet::new(&TEST_BYTES);
        let tree = packet.flatnode_tree();
        let table = packet.flatnode_prefix_table(&tree);
        let decoded_message = super::flatnode_decode_message_prefix_table(&packet, &table);
        assert_eq!(decoded_message, EXPECTED_MESSAGE);
    }

    #[test]
    fn all_samples_baseline_to_prefix_table() {
        for case in SAMPLE_CASES.iter().rev() {
            println!("case: {}_{}", case.main_category, case.sub_category);
            let content = case.request();
            let expected_result = super::treenode_decode_packet_traverse(&content);
            let result = super::flatnode_decode_packet_prefix_table(&content);
            assert_eq!(result, expected_result)
        }
    }
}

// MARK: Benches

static BENCH_SAMPLE_COUNT: u32 = 1_000_000;

#[divan::bench_group(sample_count = BENCH_SAMPLE_COUNT)]
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
    bench_decode_packet!(treenode_decode_packet_traverse_safe);
    bench_decode_packet!(flatnode_decode_packet_traverse_safe_index);
    bench_decode_packet!(flatnode_decode_packet_traverse_safe_const);
    bench_decode_packet!(treenode_decode_packet_traverse);
    bench_decode_packet!(flatnode_decode_packet_traverse);
    bench_decode_packet!(flatnode_decode_packet_prefix_table);
    bench_decode_packet!(flatnode_decode_packet_prefix_table_safe_index);
    bench_decode_packet!(flatnode_decode_packet_prefix_table_safe_const);
}

#[divan::bench_group(sample_count = BENCH_SAMPLE_COUNT)]
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
                let packet = &Packet::new(&response_bytes);
                let tree = packet.$tree();
                bencher
                    .counter(BytesCount::from(packet.decoded_bytes_len))
                    .bench_local(move || {
                        super::$name(black_box(&packet), &tree);
                    });
            }
        };
    }

    macro_rules! bench_decode_message_table {
        ($name:ident, $tree:ident, $table:ident ) => {
            #[divan::bench(args = ALL_CASES)]
            fn $name(bencher: Bencher, case: &Case) {
                let response_bytes = case.request();
                let packet = &Packet::new(&response_bytes);
                let tree = packet.$tree();
                let table = packet.$table(&tree);
                bencher
                    .counter(BytesCount::from(packet.decoded_bytes_len))
                    .bench_local(move || {
                        super::$name(black_box(&packet), &table);
                    });
            }
        };
    }

    bench_decode_message!(treenode_decode_message_traverse_baseline, treenode_tree);
    bench_decode_message!(treenode_decode_message_traverse_safe, treenode_tree);
    bench_decode_message!(treenode_decode_message_traverse, treenode_tree);
    bench_decode_message!(flatnode_decode_message_traverse, flatnode_tree);
    bench_decode_message!(
        flatnode_decode_message_traverse_safe_index,
        flatnode_tree_safe
    );
    bench_decode_message!(flatnode_decode_message_traverse_safe_const, flatnode_tree);
    bench_decode_message_table!(
        flatnode_decode_message_prefix_table,
        flatnode_tree,
        flatnode_prefix_table
    );
    bench_decode_message_table!(
        flatnode_decode_message_prefix_table_safe,
        flatnode_tree_safe,
        flatnode_prefix_table_safe_index
    );
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

#[divan::bench_group(sample_count = BENCH_SAMPLE_COUNT)]
mod decode_packet_group_large {
    use crate::test_cases::*;
    use divan::{black_box, Bencher};

    static CASE: &Case = &ALL_CASES[0];
    bench_group_decode_packet!(treenode_decode_packet_traverse_baseline);
    bench_group_decode_packet!(treenode_decode_packet_traverse_safe);
    bench_group_decode_packet!(flatnode_decode_packet_traverse_safe_index);
    bench_group_decode_packet!(flatnode_decode_packet_traverse_safe_const);
    bench_group_decode_packet!(treenode_decode_packet_traverse);
    bench_group_decode_packet!(flatnode_decode_packet_traverse);
    bench_group_decode_packet!(flatnode_decode_packet_prefix_table);
    bench_group_decode_packet!(flatnode_decode_packet_prefix_table_safe_index);
    bench_group_decode_packet!(flatnode_decode_packet_prefix_table_safe_const);
}

#[divan::bench_group(sample_count = BENCH_SAMPLE_COUNT)]
mod decode_packet_group_large_medium {
    use crate::test_cases::*;
    use divan::{black_box, Bencher};

    static CASE: &Case = &ALL_CASES[1];

    bench_group_decode_packet!(treenode_decode_packet_traverse_baseline);
    bench_group_decode_packet!(treenode_decode_packet_traverse_safe);
    bench_group_decode_packet!(flatnode_decode_packet_traverse_safe_index);
    bench_group_decode_packet!(flatnode_decode_packet_traverse_safe_const);
    bench_group_decode_packet!(treenode_decode_packet_traverse);
    bench_group_decode_packet!(flatnode_decode_packet_traverse);
    bench_group_decode_packet!(flatnode_decode_packet_prefix_table);
    bench_group_decode_packet!(flatnode_decode_packet_prefix_table_safe_index);
    bench_group_decode_packet!(flatnode_decode_packet_prefix_table_safe_const);
}

#[divan::bench_group(sample_count = BENCH_SAMPLE_COUNT)]
mod decode_packet_group_medium {
    use crate::test_cases::*;
    use divan::{black_box, Bencher};

    static CASE: &Case = &ALL_CASES[2];

    bench_group_decode_packet!(treenode_decode_packet_traverse_baseline);
    bench_group_decode_packet!(treenode_decode_packet_traverse_safe);
    bench_group_decode_packet!(flatnode_decode_packet_traverse_safe_index);
    bench_group_decode_packet!(flatnode_decode_packet_traverse_safe_const);
    bench_group_decode_packet!(treenode_decode_packet_traverse);
    bench_group_decode_packet!(flatnode_decode_packet_traverse);
    bench_group_decode_packet!(flatnode_decode_packet_prefix_table);
    bench_group_decode_packet!(flatnode_decode_packet_prefix_table_safe_index);
    bench_group_decode_packet!(flatnode_decode_packet_prefix_table_safe_const);
}

#[divan::bench_group(sample_count = BENCH_SAMPLE_COUNT)]
mod decode_packet_group_medium_small {
    use crate::test_cases::*;
    use divan::{black_box, Bencher};

    static CASE: &Case = &ALL_CASES[3];

    bench_group_decode_packet!(treenode_decode_packet_traverse_baseline);
    bench_group_decode_packet!(treenode_decode_packet_traverse_safe);
    bench_group_decode_packet!(flatnode_decode_packet_traverse_safe_index);
    bench_group_decode_packet!(flatnode_decode_packet_traverse_safe_const);
    bench_group_decode_packet!(treenode_decode_packet_traverse);
    bench_group_decode_packet!(flatnode_decode_packet_traverse);
    bench_group_decode_packet!(flatnode_decode_packet_prefix_table);
    bench_group_decode_packet!(flatnode_decode_packet_prefix_table_safe_index);
    bench_group_decode_packet!(flatnode_decode_packet_prefix_table_safe_const);
}

#[divan::bench_group(sample_count = BENCH_SAMPLE_COUNT)]
mod decode_packet_group_small {
    use crate::test_cases::*;
    use divan::{black_box, Bencher};

    static CASE: &Case = &ALL_CASES[4];

    bench_group_decode_packet!(treenode_decode_packet_traverse_baseline);
    bench_group_decode_packet!(treenode_decode_packet_traverse_safe);
    bench_group_decode_packet!(flatnode_decode_packet_traverse_safe_index);
    bench_group_decode_packet!(flatnode_decode_packet_traverse_safe_const);
    bench_group_decode_packet!(treenode_decode_packet_traverse);
    bench_group_decode_packet!(flatnode_decode_packet_traverse);
    bench_group_decode_packet!(flatnode_decode_packet_prefix_table);
    bench_group_decode_packet!(flatnode_decode_packet_prefix_table_safe_index);
    bench_group_decode_packet!(flatnode_decode_packet_prefix_table_safe_const);
}

#[divan::bench_group(sample_count = BENCH_SAMPLE_COUNT)]
mod decode_packet_group_small_min {
    use crate::test_cases::*;
    use divan::{black_box, Bencher};

    static CASE: &Case = &ALL_CASES[5];

    bench_group_decode_packet!(treenode_decode_packet_traverse_baseline);
    bench_group_decode_packet!(treenode_decode_packet_traverse_safe);
    bench_group_decode_packet!(flatnode_decode_packet_traverse_safe_index);
    bench_group_decode_packet!(flatnode_decode_packet_traverse_safe_const);
    bench_group_decode_packet!(treenode_decode_packet_traverse);
    bench_group_decode_packet!(flatnode_decode_packet_traverse);
    bench_group_decode_packet!(flatnode_decode_packet_prefix_table);
    bench_group_decode_packet!(flatnode_decode_packet_prefix_table_safe_index);
    bench_group_decode_packet!(flatnode_decode_packet_prefix_table_safe_const);
}
