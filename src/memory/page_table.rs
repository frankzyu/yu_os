use crate::address::*;
use bitflags::bitflags;

use core::convert::TryInto;
use core::fmt::{Debug, Error, Formatter};
use core::marker::PhantomData;
use core::ops::{Index, IndexMut};
unsafe fn sfence_vma(asid: usize, va: usize) {
    core::arch::asm!("sfence.vma {0}, {1}", in(reg) asid, in(reg) va);
}
pub type Entries64 = [PageTableEntryX64; RV64_ENTRY_COUNT];

// To avoid const generic.
pub trait PTEIterableSlice<T> {
    fn to_pte_slice<'a>(&'a self) -> &'a [T];
    fn to_pte_slice_mut<'a>(&'a mut self) -> &'a mut [T];
    fn pte_index(&self, index: usize) -> &T;
    fn pte_index_mut(&mut self, index: usize) -> &mut T;
}

impl PTEIterableSlice<PageTableEntryX64> for Entries64 {
    fn to_pte_slice(&self) -> &[PageTableEntryX64] {
        self
    }
    fn to_pte_slice_mut(&mut self) -> &mut [PageTableEntryX64] {
        self
    }
    fn pte_index(&self, index: usize) -> &PageTableEntryX64 {
        &self[index]
    }
    fn pte_index_mut(&mut self, index: usize) -> &mut PageTableEntryX64 {
        &mut self[index]
    }
}

#[repr(C)]
pub struct PageTableWith<T: PTEIterableSlice<E>, E: PTE> {
    entries: T,
    phantom: PhantomData<E>,
}

impl<T: PTEIterableSlice<E>, E: PTE> PageTableWith<T, E> {
    /// Clears all entries.
    pub fn zero(&mut self) {
        for entry in self.entries.to_pte_slice_mut().iter_mut() {
            entry.set_unused();
        }
    }
}

impl<T: PTEIterableSlice<E>, E: PTE> Index<usize> for PageTableWith<T, E> {
    type Output = E;

    fn index(&self, index: usize) -> &Self::Output {
        self.entries.pte_index(index)
    }
}

impl<T: PTEIterableSlice<E>, E: PTE> IndexMut<usize> for PageTableWith<T, E> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.entries.pte_index_mut(index)
    }
}

impl<T: PTEIterableSlice<E>, E: PTE + Debug> Debug for PageTableWith<T, E> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_map()
            .entries(
                self.entries
                    .to_pte_slice()
                    .iter()
                    .enumerate()
                    .filter(|p| !p.1.is_unused()),
            )
            .finish()
    }
}

pub trait PTE {
    fn is_unused(&self) -> bool;
    fn set_unused(&mut self);
    fn flags(&self) -> PageTableFlags;
    fn ppn(&self) -> usize;
    fn ppn_u64(&self) -> u64;
    fn addr<T: PhysicalAddress + Clone + AddressX64>(&self) -> T;
    fn frame<T: PhysicalAddress + Clone + AddressX64>(&self) -> FrameWith<T>;
    fn set<T: PhysicalAddress + Clone + AddressX64>(&mut self, frame: FrameWith<T>, flags: PageTableFlags);
    fn flags_mut(&mut self) -> &mut PageTableFlags;
}

#[derive(Copy, Clone)]
pub struct PageTableEntryX64(u64);

impl PTE for PageTableEntryX64 {
    fn is_unused(&self) -> bool {
        self.0 == 0
    }
    fn set_unused(&mut self) {
        self.0 = 0;
    }
    fn flags(&self) -> PageTableFlags {
        PageTableFlags::from_bits_truncate(self.0 as usize)
    }
    fn ppn(&self) -> usize {
        self.ppn_u64().try_into().unwrap()
    }
    fn ppn_u64(&self) -> u64 {
        (self.0 >> 10) as u64
    }
    fn addr<T: PhysicalAddress + Clone + AddressX64>(&self) -> T {
        T::new_u64((self.ppn() as u64) << 12)
    }
    fn frame<T: PhysicalAddress + Clone + AddressX64>(&self) -> FrameWith<T> {
        FrameWith::of_addr(self.addr())
    }
    fn set<T: PhysicalAddress + Clone + AddressX64>(&mut self, frame: FrameWith<T>, mut flags: PageTableFlags) {
        // U540 will raise page fault when accessing page with A=0 or D=0
        flags |= EF::ACCESSED | EF::DIRTY;
        self.0 = ((frame.number() << 10) | flags.bits()) as u64;
    }
    fn flags_mut(&mut self) -> &mut PageTableFlags {
        unsafe { &mut *(self as *mut _ as *mut PageTableFlags) }
    }
}

pub struct PageTableEntryX64Printer<'a, P: PhysicalAddress + Clone + AddressX64 + Debug>(
    &'a PageTableEntryX64,
    PhantomData<*const P>,
);

impl<'a, P: PhysicalAddress + Clone + AddressX64 + Debug> Debug for PageTableEntryX64Printer<'a, P> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_struct("PageTableEntryX64")
            .field("frame", &self.0.frame::<P>())
            .field("flags", &self.0.flags())
            .finish()
    }
}

impl PageTableEntryX64 {
    pub fn new() -> Self {
        PageTableEntryX64(0)
    }

    pub fn debug_sv39<'a>(&'a self) -> PageTableEntryX64Printer<'a, PhysAddrSv39> {
        PageTableEntryX64Printer(self, PhantomData)
    }
}

impl Debug for PageTableEntryX64 {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        self.debug_sv39().fmt(f)
    }
}

pub const RV64_ENTRY_COUNT: usize = 1 << 9;

pub const ENTRY_COUNT: usize = RV64_ENTRY_COUNT;

pub type PageTableEntry = PageTableEntryX64;

pub type Entries = Entries64;

pub type PageTableX64 = PageTableWith<Entries64, PageTableEntryX64>;

pub type PageTable = PageTableX64;

bitflags! {
    pub struct PageTableFlags: usize {
        const VALID =       1 << 0;
        const READABLE =    1 << 1;
        const WRITABLE =    1 << 2;
        const EXECUTABLE =  1 << 3;
        const USER =        1 << 4;
        const GLOBAL =      1 << 5;
        const ACCESSED =    1 << 6;
        const DIRTY =       1 << 7;
        const RESERVED1 =   1 << 8;
        const RESERVED2 =   1 << 9;
    }
}

type EF = PageTableFlags;

// A trait for types that can allocate a frame of memory.
pub trait FrameAllocatorFor<P: PhysicalAddress + Clone + AddressX64> {
    /// Allocate a frame of the appropriate size and return it if possible.
    fn alloc(&mut self) -> Option<FrameWith<P>>;
}

/// A trait for types that can deallocate a frame of memory.
pub trait FrameDeallocatorFor<P: PhysicalAddress + Clone + AddressX64> {
    /// Deallocate the given frame of memory.
    fn dealloc(&mut self, frame: FrameWith<P>);
}



pub trait FrameAllocator {
    fn alloc(&mut self) -> Option<Frame>;
}

pub trait FrameDeallocator {
    fn dealloc(&mut self, frame: Frame);
}

impl<T: FrameAllocator> FrameAllocatorFor<PhysAddr> for T {
    #[inline]
    fn alloc(&mut self) -> Option<Frame> {
        FrameAllocator::alloc(self)
    }
}

impl<T: FrameDeallocator> FrameDeallocatorFor<PhysAddr> for T {
    #[inline]
    fn dealloc(&mut self, frame: Frame) {
        FrameDeallocator::dealloc(self, frame)
    }
}

pub trait Mapper {
    type P: PhysicalAddress + Clone + AddressX64;
    type V: VirtualAddress + Clone + AddressX64;
    type MapperFlush: MapperFlushable;
    type Entry: PTE;

    /// Creates a new mapping in the page table.
    ///
    /// This function might need additional physical frames to create new page tables. These
    /// frames are allocated from the `allocator` argument. At most three frames are required.
    fn map_to(
        &mut self,
        page: PageWith<Self::V>,
        frame: FrameWith<Self::P>,
        flags: PageTableFlags,
        allocator: &mut impl FrameAllocatorFor<<Self as Mapper>::P>,
    ) -> Result<Self::MapperFlush, MapToError>;

    /// Removes a mapping from the page table and returns the frame that used to be mapped.
    ///
    /// Note that no page tables or pages are deallocated.
    fn unmap(
        &mut self,
        page: PageWith<Self::V>,
    ) -> Result<(FrameWith<Self::P>, Self::MapperFlush), UnmapError<<Self as Mapper>::P>>;

    /// Get the reference of the specified `page` entry
    fn ref_entry(&mut self, page: &PageWith<Self::V>) -> Result<&mut Self::Entry, FlagUpdateError>;

    /// Updates the flags of an existing mapping.
    fn update_flags(
        &mut self,
        page: PageWith<Self::V>,
        flags: PageTableFlags,
    ) -> Result<Self::MapperFlush, FlagUpdateError> {
        self.ref_entry(&page).map(|e| {
            e.set(e.frame::<Self::P>(), flags);
            Self::MapperFlush::new(page)
        })
    }

    /// Return the frame that the specified page is mapped to.
    fn translate_page(&mut self, page: PageWith<Self::V>) -> Option<FrameWith<Self::P>> {
        match self.ref_entry(&page) {
            Ok(e) => {
                if e.is_unused() {
                    None
                } else {
                    Some(e.frame())
                }
            }
            Err(_) => None,
        }
    }

    /// Maps the given frame to the virtual page with the same address.
    fn identity_map(
        &mut self,
        frame: FrameWith<Self::P>,
        flags: PageTableFlags,
        allocator: &mut impl FrameAllocatorFor<<Self as Mapper>::P>,
    ) -> Result<Self::MapperFlush, MapToError> {
        let page = PageWith::of_addr(Self::V::new(frame.start_address().as_usize()));
        self.map_to(page, frame, flags, allocator)
    }
}


pub trait MapperFlushable {
    /// Create a new flush promise
    fn new<T: VirtualAddress + Clone + AddressX64>(page: PageWith<T>) -> Self;
    /// Flush the page from the TLB to ensure that the newest mapping is used.
    fn flush(self);
    /// Don't flush the TLB and silence the â€œmust be usedâ€? warning.
    fn ignore(self);
}


#[must_use = "Page Table changes must be flushed or ignored."]
pub struct MapperFlush(usize);

impl MapperFlushable for MapperFlush {
    fn new<T: VirtualAddress + Clone + AddressX64>(page: PageWith<T>) -> Self {
        MapperFlush(page.start_address().as_usize())
    }
    fn flush(self) {
        unsafe {
            sfence_vma(0, self.0);
        }
    }
    fn ignore(self) {}
}


/// This error is returned from `map_to` and similar methods.
#[derive(Debug)]
pub enum MapToError {
    /// An additional frame was needed for the mapping process, but the frame allocator
    /// returned `None`.
    FrameAllocationFailed,
    /// An upper level page table entry has the `HUGE_PAGE` flag set, which means that the
    /// given page is part of an already mapped huge page.
    ParentEntryHugePage,
    /// The given page is already mapped to a physical frame.
    PageAlreadyMapped,
}

/// An error indicating that an `unmap` call failed.
#[derive(Debug)]
pub enum UnmapError<P: PhysicalAddress> {
    /// An upper level page table entry has the `HUGE_PAGE` flag set, which means that the
    /// given page is part of a huge page and can't be freed individually.
    ParentEntryHugePage,
    /// The given page is not mapped to a physical frame.
    PageNotMapped,
    /// The page table entry for the given page points to an invalid physical address.
    InvalidFrameAddress(P),
}

/// An error indicating that an `update_flags` call failed.
#[derive(Debug)]
pub enum FlagUpdateError {
    /// The given page is not mapped to a physical frame.
    PageNotMapped,
}

pub trait MapperExt {
    type Page;
    type Frame;
}

impl<T: Mapper> MapperExt for T {
    type Page = PageWith<<T as Mapper>::V>;
    type Frame = FrameWith<<T as Mapper>::P>;
}


/// This struct is a three-level page table with `Mapper` trait implemented.
pub struct Rv39PageTableWith<'a, V: VirtualAddress + AddressL3, FL: MapperFlushable> {
    root_table: &'a mut PageTableX64,
    linear_offset: u64, // VA = PA + linear_offset
    phantom: PhantomData<(V, FL)>,
}
impl<'a, V, FL> Rv39PageTableWith<'a, V, FL>
where
    V: VirtualAddress + AddressL3 + Clone + AddressX64,
    FL: MapperFlushable,
{
    pub fn new(table: &'a mut PageTableX64, linear_offset: usize) -> Self {
        Rv39PageTableWith {
            root_table: table,
            linear_offset: linear_offset as u64,
            phantom: PhantomData,
        }
    }

    fn create_p1_if_not_exist(
        &mut self,
        p3_index: usize,
        p2_index: usize,
        allocator: &mut impl FrameAllocatorFor<<Self as Mapper>::P>,
    ) -> Result<&mut PageTableX64, MapToError> {
        let p2_table = if self.root_table[p3_index].is_unused() {
            let frame = allocator.alloc().ok_or(MapToError::FrameAllocationFailed)?;
            self.root_table[p3_index].set(frame.clone(), PageTableFlags::VALID);
            let p2_table: &mut PageTableX64 = unsafe { frame.as_kernel_mut(self.linear_offset) };
            p2_table.zero();
            p2_table
        } else {
            let frame = self.root_table[p3_index].frame::<PhysAddrSv39>();
            unsafe { frame.as_kernel_mut(self.linear_offset) }
        };
        if p2_table[p2_index].is_unused() {
            let frame = allocator.alloc().ok_or(MapToError::FrameAllocationFailed)?;
            p2_table[p2_index].set(frame.clone(), PageTableFlags::VALID);
            let p1_table: &mut PageTableX64 = unsafe { frame.as_kernel_mut(self.linear_offset) };
            p1_table.zero();
            Ok(p1_table)
        } else {
            let frame = p2_table[p2_index].frame::<PhysAddrSv39>();
            let p1_table: &mut PageTableX64 = unsafe { frame.as_kernel_mut(self.linear_offset) };
            Ok(p1_table)
        }
    }
}


impl<'a, V, FL> Mapper for Rv39PageTableWith<'a, V, FL>
where
    V: VirtualAddress + AddressL3 + Clone + AddressX64,
    FL: MapperFlushable,
{
    type P = PhysAddrSv39;
    type V = V;
    type MapperFlush = FL;
    type Entry = PageTableEntryX64;

    fn map_to(
        &mut self,
        page: <Self as MapperExt>::Page,
        frame: <Self as MapperExt>::Frame,
        flags: PageTableFlags,
        allocator: &mut impl FrameAllocatorFor<<Self as Mapper>::P>,
    ) -> Result<Self::MapperFlush, MapToError>
    {
        let p1_table = self.create_p1_if_not_exist(page.p3_index(), page.p2_index(), allocator)?;
        if !p1_table[page.p1_index()].is_unused() {
            return Err(MapToError::PageAlreadyMapped);
        }
        p1_table[page.p1_index()].set(frame, flags);
        Ok(Self::MapperFlush::new(page))
    }

    fn unmap(
        &mut self,
        page: <Self as MapperExt>::Page,
    ) -> Result<(<Self as MapperExt>::Frame, Self::MapperFlush), UnmapError<<Self as Mapper>::P>>
    {
        if self.root_table[page.p3_index()].is_unused() {
            return Err(UnmapError::PageNotMapped);
        }
        let p2_frame = self.root_table[page.p3_index()].frame::<PhysAddrSv39>();
        let p2_table: &mut PageTableX64 = unsafe { p2_frame.as_kernel_mut(self.linear_offset) };

        if p2_table[page.p2_index()].is_unused() {
            return Err(UnmapError::PageNotMapped);
        }
        let p1_frame = p2_table[page.p2_index()].frame::<PhysAddrSv39>();
        let p1_table: &mut PageTableX64 = unsafe { p1_frame.as_kernel_mut(self.linear_offset) };
        let p1_entry = &mut p1_table[page.p1_index()];
        if !p1_entry.flags().contains(PageTableFlags::VALID) {
            return Err(UnmapError::PageNotMapped);
        }
        let frame = p1_entry.frame();
        p1_entry.set_unused();
        Ok((frame, Self::MapperFlush::new(page)))
    }

    fn ref_entry(
        &mut self,
        page: &<Self as MapperExt>::Page,
    ) -> Result<&mut PageTableEntryX64, FlagUpdateError> {
        if self.root_table[page.p3_index()].is_unused() {
            return Err(FlagUpdateError::PageNotMapped);
        }
        let p2_frame = self.root_table[page.p3_index()].frame::<PhysAddrSv39>();
        let p2_table: &mut PageTableX64 = unsafe { p2_frame.as_kernel_mut(self.linear_offset) };
        if p2_table[page.p2_index()].is_unused() {
            return Err(FlagUpdateError::PageNotMapped);
        }

        let p1_frame = p2_table[page.p2_index()].frame::<PhysAddrSv39>();
        let p1_table: &mut PageTableX64 = unsafe { p1_frame.as_kernel_mut(self.linear_offset) };
        Ok(&mut p1_table[page.p1_index()])
    }
}

pub type Rv39PageTable<'a> = Rv39PageTableWith<'a, VirtAddrSv39, MapperFlush>;