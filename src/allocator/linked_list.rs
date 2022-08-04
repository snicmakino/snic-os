use core::{mem, ptr};
use core::alloc::{GlobalAlloc, Layout};

use crate::allocator::{align_up, Locked};

struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

pub struct LinkedListAllocator {
    head: ListNode,
}

impl LinkedListAllocator {
    pub const fn new() -> Self {
        Self {
            head: ListNode::new(0),
        }
    }

    /// 与えられたヒープ境界でアロケーターを初期化する。
    ///
    /// この関数は安全ではない。なぜなら、呼び出し側は、与えられたヒープ境界が有効であり、
    /// ヒープが未使用であることを保証しなければならないからである。
    /// このメソッドは一度だけ呼び出されなければならない。
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_free_region(heap_start, heap_size);
    }

    /// 与えられたメモリ領域をリストの先頭に追加する。
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // 開放された領域がListNodeを保持できることを確認する。
        assert_eq!(align_up(addr, mem::align_of::<ListNode>()), addr);
        assert!(size >= mem::size_of::<ListNode>());

        // ListNodeを作成し、Listの先頭に追加する
        let mut node = ListNode::new(size);
        node.next = self.head.next.take();
        let node_ptr = addr as *mut ListNode;
        node_ptr.write(node);
        self.head.next = Some(&mut *node_ptr)
    }

    /// 与えられたサイズとアライメントを持つ空き領域を探し、リストから削除する
    ///
    /// ListNodeと割り当て開始アドレスのタプルを返す
    fn find_region(&mut self, size: usize, align: usize)
                   -> Option<(&'static mut ListNode, usize)> {
        let mut current = &mut self.head;

        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) = Self::alloc_from_region(&region, size, align) {
                // 領域が割り当てに適している場合 -> リストからノードを削除
                let next = region.next.take();
                let ret = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;
                return ret;
            } else {
                // 領域が不適な場合 -> 次の領域で処理続行
                current = current.next.as_mut().unwrap();
            }
        }

        None
    }

    /// 指定されたサイズとアライメントで、指定された領域の割り当てを試みる
    ///
    /// 成功した場合、割り当て開始アドレスを返却する
    fn alloc_from_region(region: &ListNode, size: usize, align: usize)
                         -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            // 領域が小さい
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<ListNode>() {
            // 残りの領域が小さすぎてListNodeを保持できない
            // （アロケーションで使用領域と空き領域に分割されるため必要）
            return Err(());
        }

        Ok(alloc_start)
    }

    /// 与えられたレイアウトを調整する
    /// 結果として割り当てられたメモリー領域は`ListNode`を格納することができる
    ///
    /// 調整後のサイズとアライメントを (size, align) タプルとして返却する
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<ListNode>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<ListNode>());
        (size, layout.align())
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (size, align) = LinkedListAllocator::size_align(layout);
        let mut allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("overflow");
            let excess_size = region.end_addr() - alloc_end;
            if excess_size > 0 {
                allocator.add_free_region(alloc_end, excess_size);
            }
            alloc_start as *mut u8
        } else {
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let (size, _) = LinkedListAllocator::size_align(layout);
        self.lock().add_free_region(ptr as usize, size)
    }
}