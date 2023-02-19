pub mod palloc;
pub mod ptable;

extern "C" {
    static _page_start: usize;
    static _page_end: usize;
}

static mut POOL: palloc::Kpools = palloc::Kpools::new(_page_start, _page_end);
