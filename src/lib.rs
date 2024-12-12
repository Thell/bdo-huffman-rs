use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

// Basic request for market data, not safe for general usage.
pub mod get_market;

// A zero-copy representation of a request packet's parsed body content.
// Packet also contains the methods for tree and prefix building from the different approaches.
pub mod packet;

// Classic min heap using vec as container and sift_up, sift_down as push, pop.
pub mod min_heap;

// TreeNode (nested Boxed children) and FlatNode (const ptr children) implementations.
pub mod node;

// Specific packet details for test and cases for benching.
pub mod test_cases;

// Decoding functions from the different approaches.
pub mod decoder;
