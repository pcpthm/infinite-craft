pub struct HyperpathIter {
    queue: Vec<u32>,
    blocked: Vec<bool>,
    stack: Vec<(usize, usize)>,
    max_depth: usize,
}

impl HyperpathIter {
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

    pub fn enqueue(&mut self, u: u32) -> bool {
        if std::mem::replace(&mut self.blocked[u as usize], true) {
            return false;
        }
        self.queue.push(u);
        true
    }
}

#[test]
fn enumerate_lattice_animals() {
    let n = 6;
    let height: usize = n;
    let width = n * 2 - 1;

    let start = 0 * width as u32 + (n - 1) as u32;
    let mut blocked = vec![false; height * width];
    for x in 0..n - 1 {
        blocked[x] = true;
    }

    let mut iter = HyperpathIter::new(n, blocked);
    iter.enqueue(start);

    let mut count = vec![0; n + 1];
    let mut path = Vec::with_capacity(n);
    while let Some(u) = iter.next(&mut path) {
        count[path.len()] += 1;
        if path.len() < n {
            let y: i32 = (u / width as u32) as i32;
            let x = (u % width as u32) as i32;
            for (dy, dx) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
                let y = y + dy;
                let x = x + dx;
                if (y as usize) < height && (x as usize) < width {
                    iter.enqueue(y as u32 * width as u32 + x as u32);
                }
            }
        }
    }
    println!("count = {:?}", count);
    assert_eq!(count[2..=6], vec![2, 6, 19, 63, 216]);
}
