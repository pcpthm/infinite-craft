//! Redelmeier's algorithm for hyperpath enumeration.

pub struct SetEnumeration {
    queue: Vec<u32>,
    blocked: Vec<bool>,
    stack: Vec<(usize, usize)>,
    max_depth: usize,
}

impl SetEnumeration {
    pub fn new(max_depth: usize, blocked: Vec<bool>) -> Self {
        let mut stack = Vec::with_capacity(max_depth + 1);
        stack.push((0, 0));

        Self {
            queue: Vec::new(),
            blocked,
            stack,
            max_depth,
        }
    }

    pub fn next(&mut self, path: &mut Vec<u32>) -> Option<u32> {
        while let Some((i, pqt)) = self.stack.pop() {
            let qt = self.queue.len();
            if i < qt {
                let u = self.queue[i];
                path.push(u);
                self.stack.push((i + 1, pqt));

                if self.stack.len() < self.max_depth {
                    self.stack.push((i + 1, qt));
                } else {
                    self.stack.push((usize::MAX, qt));
                }

                return Some(u);
            }

            for &u in &self.queue[pqt..] {
                self.blocked[u as usize] = false;
            }
            self.queue.truncate(pqt);
            if !self.stack.is_empty() {
                path.pop();
            }
        }
        None
    }

    pub fn enqueue(&mut self, u: u32) {
        if !std::mem::replace(&mut self.blocked[u as usize], true) {
            self.queue.push(u);
        }
    }
}

#[test]
fn count_fixed_polyominos() {
    let n = 6;
    let width = n * 2 - 1; // The grid is n ✕ (2n-1) and (0, n-1) is the lexcographic first block.

    let mut blocked = vec![false; n * width];
    blocked[0..n - 1].iter_mut().for_each(|b| *b = true);

    let mut iter = SetEnumeration::new(n, blocked);
    iter.enqueue((n - 1) as u32);

    let mut count = vec![0; n + 1];
    let mut path = Vec::with_capacity(n);
    while let Some(u) = iter.next(&mut path) {
        count[path.len()] += 1;
        if path.len() < n {
            iter.enqueue(u - 1);
            iter.enqueue(u + 1);
            iter.enqueue(u + width as u32);
            if width <= u as _ {
                iter.enqueue(u - width as u32);
            }
        }
    }

    assert_eq!(count[1..], vec![1, 2, 6, 19, 63, 216]);
}
