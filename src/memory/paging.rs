use crate::alloc_frame;
use crate::consts::*;
use crate::address::*;
use crate::dealloc_frame;
use crate::page_table::PageTableEntry;
use crate::page_table::PageTableFlags as EF;
use crate::page_table::PTE;
use crate::page_table::*;

use crate::register::*;
pub fn access_pa_via_va(paddr: usize) -> usize {
    paddr + PHYSICAL_MEMORY_OFFSET
}

#[repr(C, align(4096))]
pub struct PageTableEntryArray([PageTableEntry; 512]);

impl PageTableEntryArray {
    pub fn zero(&mut self) {
        for entry in self.0.iter_mut() {
            *entry = PageTableEntry::new();
        }
    }
}

// 定义内联汇编函数
#[inline(always)]
unsafe fn sfence_vma(asid: usize, va: usize) {
    core::arch::asm!("sfence.vma {0}, {1}", in(reg) asid, in(reg) va);
}

#[inline(always)]
unsafe fn sfence_vma_all() {
    core::arch::asm!("sfence.vma zero, zero");
}

pub struct PageEntry(&'static mut PageTableEntry, Page);

impl PageEntry {
    pub fn update(&mut self) {
        unsafe {
            sfence_vma(0, self.1.start_address().as_usize());
        }
    }

    pub fn accessed(&self) -> bool { self.0.flags().contains(EF::ACCESSED) }
    pub fn clear_accessed(&mut self) { self.0.flags_mut().remove(EF::ACCESSED); }

    pub fn dirty(&self) -> bool { self.0.flags().contains(EF::DIRTY) }
    pub fn clear_dirty(&mut self) { self.0.flags_mut().remove(EF::DIRTY); }

    pub fn writable(&self) -> bool { self.0.flags().contains(EF::WRITABLE) }
    pub fn set_writable(&mut self, value: bool) {
        self.0.flags_mut().set(EF::WRITABLE, value); 
    }

    pub fn present(&self) -> bool { self.0.flags().contains(EF::VALID | EF::READABLE) }
    pub fn set_present(&mut self, value: bool) {
        self.0.flags_mut().set(EF::VALID | EF::READABLE, value);
    }

    pub fn user(&self) -> bool { self.0.flags().contains(EF::USER) }
    pub fn set_user(&mut self, value: bool) { self.0.flags_mut().set(EF::USER, value); }

    pub fn execute(&self) -> bool { self.0.flags().contains(EF::EXECUTABLE) }
    pub fn set_execute(&mut self, value: bool) {
        self.0.flags_mut().set(EF::EXECUTABLE, value);
    }

    pub fn target(&self) -> usize {
        self.0.addr::<PhysAddrSv39>().as_usize()
    }
    pub fn set_target(&mut self, target: usize) {
        let flags = self.0.flags();
        let frame = Frame::of_addr(PhysAddr::new(target));
        self.0.set(frame, flags);
    }
}

struct FrameAllocatorForPaging;
pub trait FrameAllocator {
    fn alloc(&mut self) -> Option<Frame>;
}
impl FrameAllocator for FrameAllocatorForPaging {
    fn alloc(&mut self) -> Option<Frame> {
        alloc_frame()
    }
}
pub trait FrameDeallocator {
    fn dealloc(&mut self, frame: Frame);
}
impl FrameDeallocator for FrameAllocatorForPaging {
    fn dealloc(&mut self, frame: Frame) {
        dealloc_frame(frame)
    }
}

pub struct PageTableImpl {
    page_table: Rv39PageTable<'static>,
    root_frame: Frame,
    entry: Option<PageEntry>,
}

impl PageTableImpl {
    pub fn new_bare() -> Self {
        let frame = alloc_frame().expect("alloc_frame failed!");
        let paddr = frame.start_address().as_usize();
        let table = unsafe { &mut *(access_pa_via_va(paddr) as *mut PageTableEntryArray) };
        table.zero();
        let page_table: &mut PageTableWith<Entries64, PageTableEntryX64> = unsafe {
            &mut *(table as *mut _ as *mut PageTableWith<Entries64, PageTableEntryX64>)
        };
        PageTableImpl {
            page_table: Rv39PageTable::new(page_table, PHYSICAL_MEMORY_OFFSET),
            root_frame: frame,
            entry: None
        }
    }

    pub fn map(&mut self, va: usize, pa: usize) -> &mut PageEntry {
        let flags = EF::VALID | EF::READABLE | EF::WRITABLE;
        let page = Page::of_addr(VirtAddr::new(va));
        let frame = Frame::of_addr(PhysAddr::new(pa));
        self.page_table
            .map_to(page, frame, flags, &mut FrameAllocatorForPaging)
            .unwrap()
            .flush();
        self.get_entry(va).expect("fail to get an entry!")
    }

    pub fn unmap(&mut self, va: usize) {
        let page = Page::of_addr(VirtAddr::new(va));
        let (_, flush) = self.page_table.unmap(page).unwrap();
        flush.flush();
    }

    fn get_entry(&mut self, va: usize) -> Option<&mut PageEntry> {
        let page = Page::of_addr(VirtAddr::new(va));
        if let Ok(e) = self.page_table.ref_entry(page.clone()) {
            let e = unsafe { &mut *(e as *mut PageTableEntry) };
            self.entry = Some(PageEntry(e, page));
            self.entry.as_mut()
        } else {
            None
        }
    }
    pub fn token(&self) -> usize { self.root_frame.number() | (8 << 60) }

    unsafe fn set_token(token: usize) {
        core::arch::asm!("csrw satp, {}", in(reg) token);
    }

    fn active_token() -> usize { satp::read().bits() }

    fn flush_tlb() { unsafe { sfence_vma_all(); } }

    pub unsafe fn activate(&self) {
        let old_token = Self::active_token();
        let new_token = self.token();
        println!("switch satp from {:#x} to {:#x}", old_token, new_token);
        if new_token != old_token {
            Self::set_token(new_token);
            Self::flush_tlb();
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct PageRange {
    start: usize,
    end: usize
}

// 为 PageRange 实现 Iterator trait 成为可被遍历的迭代器
impl Iterator for PageRange {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        if self.start < self.end {
            let page = self.start << 12;
            self.start += 1;
            Some(page)
        } else {
            None
        }
    }
}

impl PageRange {
    pub fn new(start_addr: usize, end_addr: usize) -> Self {
        PageRange {
            start: start_addr / PAGE_SIZE,
            end: (end_addr - 1) / PAGE_SIZE + 1
        }
    }
}



