use std::collections::HashSet;

use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator,
};

use crate::{Recipe, SymPair, NOTHING};

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
    pub fn range(&self) -> (usize, usize) {
        (self.head, self.buf.len())
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
    d: usize,
    queue: &mut Queue,
    set: &mut Vec<u32>,
    recipe: &Recipe,
    cb: &mut impl FnMut(&Queue, &[u32]),
) {
    if d == 0 {
        cb(queue, set);
        return;
    }
    let (head, tail) = queue.range();
    while let Some(u) = queue.dequeue() {
        set.push(u);
        for &v in set.iter() {
            if let Some(w) = recipe.get(u, v) {
                queue.enqueue(w);
            }
        }
        enum_set_rec(d - 1, queue, set, recipe, cb);
        set.pop();
        queue.truncate(tail);
    }
    queue.head = head;
}

fn get_n(init: &[u32], recipe: &Recipe) -> usize {
    init.iter().fold(recipe.max_item, |acc, &u| acc.max(u)) as usize + 1
}

pub fn enum_set(d: usize, init: &[u32], recipe: &Recipe, cb: &mut impl FnMut(&Queue, &[u32])) {
    assert!(d > 0);
    let mut queue = Queue::new(get_n(init, recipe), init);
    let mut set = Vec::new();
    for &u in init {
        set.push(u);
        for &v in &set {
            if let Some(w) = recipe.get(u, v) {
                queue.enqueue(w);
            }
        }
    }
    enum_set_rec(d - 1, &mut queue, &mut set, recipe, cb);
}

fn collect_new_pairs_leaf(
    first: &[u32],
    second: &[u32],
    recipe: &Recipe,
    new_pairs: &mut HashSet<SymPair>,
) {
    for &u in first {
        for &v in second {
            if !recipe.contains(u, v) {
                new_pairs.insert(SymPair::new(u, v));
            }
        }
        if !recipe.contains(u, u) {
            new_pairs.insert(SymPair::new(u, u));
        }
    }
}

pub fn collect_new_pairs(
    depth: usize,
    init: &[u32],
    recipe: &Recipe,
    new_pairs: &mut HashSet<SymPair>,
) -> u64 {
    if depth == 0 {
        collect_new_pairs_leaf(init, init, recipe, new_pairs);
        return 1;
    }

    let pd = depth.min(5);
    let mut states = Vec::new();
    enum_set(pd, init, recipe, &mut |queue, set| {
        states.push((queue.buf.as_slice().to_owned(), queue.head, set.to_owned()));
    });

    let mut new_pairs_list = vec![HashSet::new(); states.len()];
    let count = states
        .into_par_iter()
        .zip(new_pairs_list.par_iter_mut())
        .map(|((buf, head, mut set), new_pairs)| {
            let mut queue = Queue::from_buf(buf, head, get_n(init, recipe), init);
            let mut count = 0u64;
            enum_set_rec(
                depth - pd,
                &mut queue,
                &mut set,
                recipe,
                &mut |queue, set| {
                    collect_new_pairs_leaf(queue.as_slice(), set, recipe, new_pairs);
                    count += 1;
                },
            );
            count
        })
        .sum::<u64>();
    new_pairs.extend(new_pairs_list.into_iter().flatten());
    count
}
