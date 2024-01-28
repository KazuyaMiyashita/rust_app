#![cfg_attr(not(test), no_std)]

use core::marker::Copy;

#[derive(Debug, PartialEq)]
pub struct FixedSizePriorityQueue<T, const N: usize> {
    pub array: [Option<T>; N], // led_pins のテストのため。。
    size: usize,
}

impl<T, const N: usize> FixedSizePriorityQueue<T, N>
where
    T: Copy + Ord + Sized,
{
    pub fn new() -> Self {
        FixedSizePriorityQueue {
            array: [None; N],
            size: 0,
        }
    }

    pub fn peek(&self) -> Option<&T> {
        self.array.get(0).and_then(|opt| opt.as_ref())
    }

    // 先頭を取り出して、末尾を先頭に持って行って、先頭からmin_heapy
    pub fn pop(&mut self) -> Option<T> {
        if self.size > 0 {
            let root = self.array[0].take().unwrap();
            let li = self.size - 1;
            if let Some(_) = self.array[li] {
                self.array.swap(0, li);
                self.min_heapy(0);
            }
            self.size -= 1;
            Some(root)
        } else {
            None
        }
    }

    pub fn push(&mut self, item: T) -> bool {
        if self.size < N {
            self.array[self.size] = Some(item);
            self.heapify_up(self.size);
            self.size += 1;
            true
        } else {
            false // もう追加できない
        }
    }

    // fn root() -> usize {
    //     0
    // }
    fn parent(i: usize) -> Option<usize> {
        if i == 0 {
            None
        } else {
            Some((i + 1) / 2 - 1)
        }
    }
    fn left(i: usize) -> Option<usize> {
        // FIXME: iが大きすぎる時にオーバーフロー
        Some((i + 1) * 2 - 1).filter(|&i| i < N)
    }
    fn right(i: usize) -> Option<usize> {
        // FIXME: iが大きすぎる時にオーバーフロー
        Some((i + 1) * 2).filter(|&i| i < N)
    }

    // インデックスで指定されたノードとその子ノードの間でヒープの条件を満たすようにする
    fn min_heapy(&mut self, i: usize) {
        let _ = self.array[i].unwrap();

        let smallest: usize = [Some(i), Self::left(i), Self::right(i)]
            .iter()
            .filter_map(|&option| option)
            .flat_map(|i| self.array[i].map(|v| (v, i)))
            .min_by(|x, y| x.0.cmp(&y.0))
            .unwrap()
            .1;

        if smallest != i {
            self.array.swap(i, smallest);
            self.min_heapy(smallest)
        }
    }

    // indexで指定した子の値が親の値よりが小さければ入れ替えて、根の方向に繰り返す
    fn heapify_up(&mut self, ci: usize) {
        let cv = self.array[ci].unwrap();
        if let Some(pi) = Self::parent(ci) {
            let pv = self.array[pi].unwrap();
            if cv < pv {
                self.array.swap(ci, pi);
                self.heapify_up(pi)
            }
        }
    }
}

#[cfg(test)]
#[test]
fn test1() {
    const N: usize = 3;
    let mut queue = FixedSizePriorityQueue::<u32, N>::new();
    assert_eq!(
        queue,
        FixedSizePriorityQueue {
            array: [None, None, None],
            size: 0
        }
    );

    assert_eq!(queue.pop(), None);
    assert_eq!(
        queue,
        FixedSizePriorityQueue {
            array: [None, None, None],
            size: 0
        }
    );

    assert_eq!(queue.pop(), None);
    assert_eq!(
        queue,
        FixedSizePriorityQueue {
            array: [None, None, None],
            size: 0
        }
    );

    assert_eq!(queue.push(1), true);
    assert_eq!(
        queue,
        FixedSizePriorityQueue {
            array: [Some(1), None, None],
            size: 1
        }
    );

    assert_eq!(queue.pop(), Some(1));
    assert_eq!(
        queue,
        FixedSizePriorityQueue {
            array: [None, None, None],
            size: 0
        }
    );

    assert_eq!(queue.push(1), true);
    assert_eq!(queue.push(3), true);
    assert_eq!(
        queue,
        FixedSizePriorityQueue {
            array: [Some(1), Some(3), None],
            size: 2
        }
    );
    assert_eq!(queue.pop(), Some(1));
    assert_eq!(
        queue,
        FixedSizePriorityQueue {
            array: [Some(3), None, None],
            size: 1
        }
    );

    assert_eq!(queue.pop(), Some(3));
    assert_eq!(
        queue,
        FixedSizePriorityQueue {
            array: [None, None, None],
            size: 0
        }
    );

    assert_eq!(queue.push(1), true);
    assert_eq!(queue.push(3), true);
    assert_eq!(queue.push(2), true);
    assert_eq!(queue.push(4), false);
    assert_eq!(
        queue,
        FixedSizePriorityQueue {
            array: [Some(1), Some(3), Some(2)], // 子の左右が左 <= 右とは限らない
            size: 3
        }
    );
    assert_eq!(queue.pop(), Some(1));
    assert_eq!(queue.pop(), Some(2));
    assert_eq!(queue.pop(), Some(3));
}
