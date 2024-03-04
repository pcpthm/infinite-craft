use std::collections::HashSet;

use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator,
};

use crate::{Pair, RecipeSet, NOTHING};

pub struct Queue {
    buf: Vec<u32>,
    head: usize,
    blocked: Vec<bool>,
}

impl Queue {
    pub fn new(n: usize, init: &[u32]) -> Self {
        Self::from_buf(Vec::new(), 0, n, init)
    }

    fn from_buf(buf: Vec<u32>, head: usize, n: usize, init: &[u32]) -> Self {
        let mut blocked = vec![false; n];
        blocked[NOTHING as usize] = true;
        init.iter().for_each(|&u| blocked[u as usize] = true);
        buf.iter().for_each(|&u| blocked[u as usize] = true);
        Self { buf, head, blocked }
    }

    #[inline]
    pub fn as_slice(&self) -> &[u32] {
        &self.buf[self.head..]
    }

    #[inline]
    pub fn dequeue(&mut self) -> Option<u32> {
        if self.head >= self.buf.len() {
            return None;
        }
        self.head += 1;
        Some(self.buf[self.head - 1])
    }

    #[inline]
    pub fn enqueue(&mut self, u: u32) {
        if !std::mem::replace(&mut self.blocked[u as usize], true) {
            self.buf.push(u);
        }
    }

    #[inline]
    pub fn tail(&self) -> usize {
        self.buf.len()
    }

    #[inline]
    pub fn truncate(&mut self, tail: usize) {
        for &u in &self.buf[tail..] {
            self.blocked[u as usize] = false;
        }
        self.buf.truncate(tail);
    }
}

fn enum_set_rec(
    queue: &mut Queue,
    set: &mut Vec<u32>,
    recipe: &RecipeSet,
    cb: &mut impl FnMut(&Queue, &[u32]) -> bool,
) {
    if cb(queue, set) {
        return;
    }
    let (head, tail) = (queue.head, queue.tail());
    while let Some(u) = queue.dequeue() {
        set.push(u);
        for &v in set.iter() {
            if let Some(w) = recipe.get(u, v) {
                queue.enqueue(w);
            }
        }
        enum_set_rec(queue, set, recipe, cb);
        set.pop();
        queue.truncate(tail);
    }
    queue.head = head;
}

pub fn enum_set(
    n: usize,
    init: &[u32],
    recipe: &RecipeSet,
    cb: &mut impl FnMut(&Queue, &[u32]) -> bool,
) {
    let mut queue = Queue::new(n, init);
    for &u in init {
        for &v in init {
            if let Some(w) = recipe.get(u, v) {
                queue.enqueue(w);
            }
        }
    }
    enum_set_rec(&mut queue, &mut init.to_owned(), recipe, cb);
}

pub fn collect_new_pairs(
    depth: usize,
    n: usize,
    init: &[u32],
    recipe: &RecipeSet,
) -> (HashSet<Pair>, u64) {
    if depth == 0 {
        let mut new_pairs = HashSet::new();
        for &u in init {
            for &v in init {
                if !recipe.contains(u, v) {
                    new_pairs.insert(Pair::new(u, v));
                }
            }
        }
        return (new_pairs, 1);
    }

    let pd = depth.min(5);
    let mut states = Vec::new();
    enum_set(n, init, recipe, &mut |queue, set| {
        if set.len() - init.len() < pd - 1 {
            return false;
        }
        states.push((queue.buf.clone(), queue.head, set.to_owned()));
        true
    });

    let mut new_pairs_vec = vec![HashSet::new(); states.len()];
    let count = states
        .into_par_iter()
        .zip(new_pairs_vec.par_iter_mut())
        .map(|((buf, head, mut set), new_pairs)| {
            let mut queue = Queue::from_buf(buf, head, n, init);
            let mut count = 0u64;
            enum_set_rec(&mut queue, &mut set, recipe, &mut |queue, set| {
                if set.len() - init.len() < depth - 1 {
                    return false;
                }
                for &u in queue.as_slice() {
                    for &v in set {
                        if !recipe.contains(u, v) {
                            new_pairs.insert(Pair::new(u, v));
                        }
                    }
                    if !recipe.contains(u, u) {
                        new_pairs.insert(Pair::new(u, u));
                    }
                }
                count += 1;
                true
            });
            count
        })
        .sum::<u64>();

    (new_pairs_vec.into_iter().flatten().collect(), count)
}
