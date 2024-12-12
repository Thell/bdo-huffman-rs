use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
pub mod test_cases; // Specific packet details for test and cases for benching.
