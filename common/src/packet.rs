pub const MAX_SYMBOLS: usize = 12; // digits 0-9, '|' and '-'

pub struct Packet<'a> {
    pub len: u64,
    pub symbol_count: u32,
    pub symbol_table_bytes: &'a [u8],
    pub bitstream_len: u32,
    pub encoded_bytes_len: u32,
    pub decoded_bytes_len: u32,
    pub encoded_message: &'a [u8],
}

impl<'a> Packet<'a> {
    // Creates a `Packet` by taking ownership of `content`, enabling zero-copy
    // parsing to avoid allocating new storage and redundant copying.
    // This reduces the runtime of large packet parsing from ~440ns to 3.2ns.
    pub fn new(content: &'a [u8]) -> Self {
        let mut pos = 0;

        let u64_bytes: [u8; 8] = content[pos..pos + 8].try_into().unwrap();
        let len = u64::from_le_bytes(u64_bytes);
        pos += 8;

        let mut u32_bytes: [u8; 4] = content[pos..pos + 4].try_into().unwrap();
        let symbol_count = u32::from_le_bytes(u32_bytes);
        pos += 4;

        let symbol_table_bytes = &content[pos..pos + 8 * symbol_count as usize];
        pos += 8 * symbol_count as usize;

        u32_bytes = content[pos..pos + 4].try_into().unwrap();
        let bitstream_len = u32::from_le_bytes(u32_bytes);
        pos += 4;

        u32_bytes = content[pos..pos + 4].try_into().unwrap();
        let encoded_bytes_len = u32::from_le_bytes(u32_bytes);
        pos += 4;

        u32_bytes = content[pos..pos + 4].try_into().unwrap();
        let decoded_bytes_len = u32::from_le_bytes(u32_bytes);
        pos += 4;

        let encoded_message = &content[pos..pos + encoded_bytes_len as usize];

        Packet {
            len,
            symbol_count,
            symbol_table_bytes,
            bitstream_len,
            encoded_bytes_len,
            decoded_bytes_len,
            encoded_message,
        }
    }

    pub fn symbols_table(&self) -> Vec<(u8, u32)> {
        let mut frequencies = [(0u8, 0u32); 12];
        let ptr = self.symbol_table_bytes.as_ptr();
        unsafe {
            for i in 0..self.symbol_count {
                let freq_ptr = ptr.add(i as usize * 8) as *const u32;
                let symbol_ptr = ptr.add(i as usize * 8 + 4);

                let frequency = freq_ptr.read_unaligned();
                let symbol = symbol_ptr.read();

                *frequencies.get_unchecked_mut(i as usize) = (symbol, frequency);
            }
        }

        frequencies[..self.symbol_count as usize].to_vec()
    }
}

// =========================================================

// MARK: Unit Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_cases::*;

    #[test]
    fn parses_symbol_table() {
        let packet = &Packet::new(&TEST_BYTES);
        assert!(packet.symbols_table() == EXPECTED_SYMBOL_TABLE);
    }
}

// MARK: Benches
#[divan::bench_group(sample_count = 10_000)]
mod benches {
    use super::*;
    use crate::test_cases::{Case, ALL_CASES};

    use divan::{black_box, Bencher};

    #[divan::bench(args = [ALL_CASES[0], ALL_CASES[5]])]
    fn packet_from_content(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();

        bencher.bench_local(move || {
            black_box(Packet::new(response_bytes));
        });
    }

    #[divan::bench(args = [ALL_CASES[0], ALL_CASES[5]])]
    fn symbols_table(bencher: Bencher, case: &Case) {
        let response_bytes = &case.request();
        let packet = Packet::new(response_bytes);

        bencher.bench_local(move || {
            black_box(&packet.symbols_table());
        });
    }
}