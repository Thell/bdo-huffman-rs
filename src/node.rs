use crate::min_heap::MinHeapNode;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeNode {
    pub symbol: Option<u8>,
    pub frequency: u32,
    pub left_child: Option<Box<TreeNode>>,
    pub right_child: Option<Box<TreeNode>>,
}

impl MinHeapNode for TreeNode {
    fn frequency(&self) -> u32 {
        self.frequency
    }
}

impl Ord for TreeNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.frequency.cmp(&other.frequency)
    }
}
impl PartialOrd for TreeNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl TreeNode {
    pub fn new(symbol: Option<u8>, freq: u32) -> Self {
        Self {
            symbol,
            frequency: freq,
            left_child: None,
            right_child: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlatNode {
    pub symbol: Option<u8>,
    pub frequency: u32,
    pub left_ptr: *const FlatNode,
    pub right_ptr: *const FlatNode,
}

impl MinHeapNode for FlatNode {
    fn frequency(&self) -> u32 {
        self.frequency
    }
}

impl Ord for FlatNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.frequency.cmp(&other.frequency)
    }
}
impl PartialOrd for FlatNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Default for FlatNode {
    fn default() -> Self {
        Self {
            symbol: None,
            frequency: 0,
            left_ptr: std::ptr::null(),
            right_ptr: std::ptr::null(),
        }
    }
}
impl FlatNode {
    pub fn new(symbol: Option<u8>, frequency: u32) -> Self {
        Self {
            symbol,
            frequency,
            left_ptr: std::ptr::null(),
            right_ptr: std::ptr::null(),
        }
    }
}
