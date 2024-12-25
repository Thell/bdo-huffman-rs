use crate::min_heap::MinHeapNode;

///////////////////////////////////////////////////////////////////////////////
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
    fn new(symbol: Option<u8>, frequency: u32) -> Self {
        TreeNode::new(symbol, frequency)
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
    pub fn new_parent(left: Self, right: Self) -> Self {
        TreeNode {
            symbol: None,
            frequency: left.frequency + right.frequency,
            left_child: Some(Box::new(left)),
            right_child: Some(Box::new(right)),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FlatNodeSafe {
    pub left_index: u8,
    pub right_index: u8,
    pub symbol: Option<u8>,
    pub frequency: u32,
}

impl MinHeapNode for FlatNodeSafe {
    fn frequency(&self) -> u32 {
        self.frequency
    }
    fn new(symbol: Option<u8>, frequency: u32) -> Self {
        FlatNodeSafe::new(symbol, frequency)
    }
}

impl Ord for FlatNodeSafe {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.frequency.cmp(&other.frequency)
    }
}
impl PartialOrd for FlatNodeSafe {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl FlatNodeSafe {
    pub fn new(symbol: Option<u8>, frequency: u32) -> Self {
        Self {
            symbol,
            frequency,
            left_index: 0,
            right_index: 0,
        }
    }
    pub fn new_parent(frequency: u32, left_index: u8, right_index: u8) -> Self {
        Self {
            symbol: None,
            frequency,
            left_index,
            right_index,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct FlatNode {
    pub left_ptr: *const FlatNode,
    pub right_ptr: *const FlatNode,
    pub symbol: Option<u8>,
}

impl Default for FlatNode {
    fn default() -> Self {
        Self {
            left_ptr: std::ptr::null(),
            right_ptr: std::ptr::null(),
            symbol: None,
        }
    }
}
impl FlatNode {
    pub fn new(symbol: Option<u8>) -> Self {
        Self {
            left_ptr: std::ptr::null(),
            right_ptr: std::ptr::null(),
            symbol,
        }
    }
}
