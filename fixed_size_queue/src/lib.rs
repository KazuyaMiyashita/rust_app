#![cfg_attr(not(test), no_std)]

use core::marker::Copy;

pub struct FixedSizeQueue<T, const N: usize> {
    heap: [Option<T>; N],
    size: usize,
}

impl<T, const N: usize> FixedSizeQueue<T, N>
where
    T: Copy + Ord + Sized,
{
    pub fn new() -> Self {
        FixedSizeQueue {
            heap: [None; N],
            size: 0,
        }
    }

    pub fn peek(&self) -> Option<&T> {
        self.heap.get(0).and_then(|opt| opt.as_ref())
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.size > 0 {
            let root = self.heap[0].take();
            self.size -= 1;
            self.heapify_down(0);
            root
        } else {
            None
        }
    }

    pub fn push(&mut self, item: T) {
        if self.size < N {
            self.heap[self.size] = Some(item);
            self.size += 1;
            self.heapify_up(self.size - 1);
        } else if let Some(root) = self.heap.get_mut(0) {
            if let Some(top) = root.as_mut() {
                if *top < item {
                    *top = item;
                    self.heapify_down(0);
                }
            }
        }
    }

    fn heapify_up(&mut self, mut index: usize) {
        while index > 0 {
            let parent = (index - 1) / 2;
            if self.heap[index] > self.heap[parent] {
                self.heap.swap(index, parent);
                index = parent;
            } else {
                break;
            }
        }
    }

    fn heapify_down(&mut self, mut index: usize) {
        while 2 * index + 1 < self.size {
            let left_child = 2 * index + 1;
            let right_child = 2 * index + 2;
            let mut largest_child = left_child;

            if right_child < self.size && self.heap[right_child] > self.heap[left_child] {
                largest_child = right_child;
            }

            if self.heap[index] < self.heap[largest_child] {
                self.heap.swap(index, largest_child);
                index = largest_child;
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
#[test]
fn test1() {
    const N: usize = 3;
    let mut queue = FixedSizeQueue::<u32, N>::new();

    println!("{:?}", queue.heap);

    assert_eq!(queue.pop(), None);
    assert_eq!(queue.pop(), None);

    queue.push(1);
    println!("{:?}", queue.heap);

    assert_eq!(queue.pop(), Some(1));
    assert_eq!(queue.pop(), None);
    queue.push(1);
    println!("{:?}", queue.heap);

    queue.push(3);
    println!("{:?}", queue.heap);

    queue.push(2);

    println!("{:?}", queue.heap);

    assert_eq!(queue.pop(), Some(3));
    assert_eq!(queue.pop(), Some(2));
    assert_eq!(queue.pop(), Some(1));
    assert_eq!(queue.pop(), None);

    for _ in 0..N {
        queue.push(1);
    }
    queue.push(1);
    for _ in 0..N {
        assert_eq!(queue.pop(), Some(1));
    }
    assert_eq!(queue.pop(), None);
}
