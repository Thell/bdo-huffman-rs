pub struct Case {
    pub name: &'static str,
    pub main_category: u32,
    pub sub_category: u32,
}

impl std::fmt::Display for Case {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(self.name)
    }
}

impl Case {
    pub fn request(&self) -> Vec<u8> {
        let filepath = env!("CARGO_MANIFEST_DIR");
        let filepath = format!(
            "{}/sample_data/GetWorldMarketList_{}_{}.bin",
            filepath, self.main_category, self.sub_category
        );
        let filename = std::path::Path::new(&filepath);
        std::fs::read(filename).unwrap()
    }
}

#[rustfmt::skip]
pub const ALL_CASES: &[Case] = &[
    Case { name: "large (msg_len=70.5k)", main_category: 55, sub_category: 4 },
    Case { name: "large_medium (msg_len=33.3k)", main_category: 55, sub_category: 3  },
    Case { name: "medium (msg_len=22.5k)", main_category: 55, sub_category: 2  },
    Case { name: "medium_small (msg_len=11.1k)", main_category: 55, sub_category: 1  },
    Case { name: "small (msg_len=5.5k)", main_category: 25, sub_category: 2  },
    Case { name: "small_min (msg_len=40b)", main_category: 75, sub_category: 6  },
];

// The following are used in tests and benches in nested modules and will report as unused.
#[allow(unused)]
pub(crate) const TEST_BYTES: [u8; 136] = [
    136, 0, 0, 0, 0, 0, 0, 0, 12, 0, 0, 0, 6, 0, 0, 0, 45, 0, 0, 0, 11, 0, 0, 0, 48, 0, 0, 0, 3, 0,
    0, 0, 49, 0, 0, 0, 1, 0, 0, 0, 50, 0, 0, 0, 2, 0, 0, 0, 51, 0, 0, 0, 1, 0, 0, 0, 52, 0, 0, 0,
    6, 0, 0, 0, 53, 0, 0, 0, 2, 0, 0, 0, 54, 0, 0, 0, 2, 0, 0, 0, 55, 0, 0, 0, 3, 0, 0, 0, 56, 0,
    0, 0, 1, 0, 0, 0, 57, 0, 0, 0, 2, 0, 0, 0, 124, 0, 0, 0, 128, 0, 0, 0, 16, 0, 0, 0, 40, 0, 0,
    0, 229, 144, 115, 255, 244, 122, 27, 209, 242, 203, 103, 48, 153, 43, 90, 163,
];

#[allow(unused)]
pub(crate) const EXPECTED_MESSAGE: &str = "53801-0-55556-41900|53802-0-16807-70000|";

#[allow(unused)]
pub(crate) const EXPECTED_POP_ORDER: [Option<u8>; 12] = [
    Some(50),
    Some(57),
    Some(52),
    Some(54),
    Some(55),
    Some(51),
    Some(124),
    Some(56),
    Some(49),
    Some(53),
    Some(45),
    Some(48),
];

#[allow(unused)]
pub(crate) const EXPECTED_PREFIXES: [(&str, &str); 12] = [
    ("0", "10"),
    ("1", "000"),
    ("2", "110110"),
    ("3", "0010"),
    ("4", "11010"),
    ("5", "111"),
    ("6", "0100"),
    ("7", "0101"),
    ("8", "1100"),
    ("9", "110111"),
    ("-", "011"),
    ("|", "0011"),
];

#[rustfmt::skip]
#[allow(unused)]
pub(crate) const EXPECTED_SYMBOL_TABLE: [(u8, u32); 12] = [
    (45, 6), (48, 11), (49, 3), (50, 1), (51, 2), (52, 1),
    (53, 6), (54, 2), (55, 2), (56, 3), (57, 1), (124, 2),
];
