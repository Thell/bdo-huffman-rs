use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub mod min_heap;
pub mod packet;
pub mod test_cases;
