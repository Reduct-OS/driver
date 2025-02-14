pub fn malloc(len: usize, align: usize) -> isize {
    syscall!(10001, len, align)
}

pub fn free(addr: usize, len: usize, align: usize) -> isize {
    syscall!(10005, addr, len, align)
}

pub fn physmap(vaddr: usize, paddr: usize, len: usize) -> isize {
    syscall!(10002, vaddr, paddr, len)
}

use core::alloc::Layout;

#[global_allocator]
static ALLOCATOR: MemoryAllocator = MemoryAllocator;

struct MemoryAllocator;

unsafe impl alloc::alloc::GlobalAlloc for MemoryAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        malloc(layout.size(), layout.align()) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        free(ptr as usize, layout.size(), layout.align());
    }
}
