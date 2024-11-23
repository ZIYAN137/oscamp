#![no_std]

use allocator::{BaseAllocator, ByteAllocator, PageAllocator, AllocResult, AllocError};
use core::alloc::Layout;
use core::ptr::NonNull;
/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///
pub struct EarlyAllocator<const PAGE_SIZE: usize> {
    start: usize,
    end: usize,
    b_pos: usize,
    p_pos: usize,
    count: usize,
}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE> {
    pub const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            b_pos: 0,
            p_pos: 0,
            count: 0,
        }
    }
}

impl<const PAGE_SIZE: usize> BaseAllocator for EarlyAllocator<PAGE_SIZE> {
    /// Initialize the allocator with a free memory region.
    fn init(&mut self, start: usize, size: usize) {
        self.start = start;
        self.end = start + size;
        self.b_pos = self.start;
        self.p_pos = self.end;
        self.count = 0;
    }

    /// Add a free memory region to the allocator.
    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        // Do nothing
        return Ok(());
    }
}

impl<const PAGE_SIZE: usize> ByteAllocator for EarlyAllocator<PAGE_SIZE> {
    /// Allocate memory with the given size (in bytes) and alignment.
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        let size = layout.size();
        let align = layout.align();

        if size == 0 || !align.is_power_of_two() {
            return Err(AllocError::InvalidParam);
        }

        let alloc_start = (self.b_pos + (align - 1)) & !(align - 1);
        let alloc_end = alloc_start.checked_add(size).ok_or(AllocError::NoMemory)?;

        if alloc_end > self.p_pos {
            return Err(AllocError::NoMemory);
        }

        self.b_pos = alloc_end;
        self.count += 1;

        unsafe { Ok(NonNull::new_unchecked(alloc_start as *mut u8)) }
    }

    /// Deallocate memory at the given position, size, and alignment.
    fn dealloc(&mut self, _pos: NonNull<u8>, _layout: Layout) {
        if self.count > 0 {
            self.count -= 1;
            if self.count == 0 {
                self.b_pos = self.start;
            }
        }
    }

    /// Returns total memory size in bytes.
    fn total_bytes(&self) -> usize {
        self.p_pos - self.start
    }

    /// Returns allocated memory size in bytes.
    fn used_bytes(&self) -> usize {
        self.b_pos - self.start
    }

    /// Returns available memory size in bytes.
    fn available_bytes(&self) -> usize {
        self.p_pos - self.b_pos
    }
}

impl<const PAGE_SIZE: usize> PageAllocator for EarlyAllocator<PAGE_SIZE> {
    /// The size of a memory page.
    const PAGE_SIZE: usize = PAGE_SIZE;

    /// Allocate contiguous memory pages with given count and alignment.
    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> AllocResult<usize> {
        if num_pages == 0 || !align_pow2.is_power_of_two() {
            return Err(AllocError::InvalidParam);
        }

        let align = align_pow2 * Self::PAGE_SIZE;
        let alloc_size = num_pages * Self::PAGE_SIZE;

        let alloc_start = (self.p_pos - alloc_size) & !(align - 1);

        if alloc_start < self.b_pos || alloc_start.checked_add(alloc_size).unwrap_or(0) > self.p_pos {
            return Err(AllocError::NoMemory);
        }

        self.p_pos = alloc_start;

        Ok(alloc_start)
    }

    /// Deallocate contiguous memory pages with given position and count.
    fn dealloc_pages(&mut self, pos: usize, num_pages: usize) {
        // Do nothing
    }

    /// Returns the total number of memory pages.
    fn total_pages(&self) -> usize {
        (self.end - self.b_pos) / Self::PAGE_SIZE
    }

    /// Returns the number of allocated memory pages.
    fn used_pages(&self) -> usize {
        (self.end - self.p_pos) / Self::PAGE_SIZE
    }

    /// Returns the number of available memory pages.
    fn available_pages(&self) -> usize {
        self.total_pages() - self.used_pages()
    }
}
