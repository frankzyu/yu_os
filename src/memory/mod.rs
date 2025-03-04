
pub trait Address: core::fmt::Debug + Copy + Clone + PartialEq + Eq + PartialOrd + Ord {
    fn new(addr: usize) -> Self;
    fn page_number(&self) -> usize;
    fn page_offset(&self) -> usize;
    fn to_4k_aligned(&self) -> Self;
    fn as_usize(&self) -> usize;
}

pub trait VirtualAddress: Address {
    unsafe fn as_mut<'a, 'b, T>(&'a self) -> &'b mut T;
}

pub trait AddressX32: Address {
    fn new_u32(addr: u32) -> Self;
    fn as_u32(&self) -> u32;
}
pub trait AddressX64: Address {
    fn new_u64(addr: u64) -> Self;
    fn as_u64(&self) -> u64;
}

pub trait PhysicalAddress: AddressX64 {}

pub trait AddressL3: Address {
    fn p3_index(&self) -> usize;
    fn p2_index(&self) -> usize;
    fn p1_index(&self) -> usize;
    fn from_page_table_indices(
        p3_index: usize,
        p2_index: usize,
        p1_index: usize,
        offset: usize,
    ) -> Self;
}


#[macro_use]
pub mod linked_list;
mod frame_allocator;
mod buddy_system_allocator;
pub mod address;
pub mod page_table;
pub mod paging;


use buddy_system_allocator::LockedHeap;
use frame_allocator::SEGMENT_TREE_ALLOCATOR as FRAME_ALLOCATOR;
use address::Frame;
use crate::consts::*;




pub fn init(l: usize, r: usize) {
    FRAME_ALLOCATOR.lock().init(l, r);
    init_heap();
    println!("++++ setup memory!    ++++");
}

pub fn alloc_frame() -> Option<Frame> {
    Some(Frame::of_ppn(FRAME_ALLOCATOR.lock().alloc()))
}

pub fn dealloc_frame(f: Frame) {
    FRAME_ALLOCATOR.lock().dealloc(f.number())
}

fn init_heap() {
    static mut HEAP: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];
    unsafe {
        DYNAMIC_ALLOCATOR
            .lock()
            .init(HEAP.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
}

#[global_allocator]
static DYNAMIC_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(_: core::alloc::Layout) -> ! {
    panic!("alloc_error_handler do nothing but panic!");
}