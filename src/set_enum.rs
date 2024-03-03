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
        Self::from_buf(init.to_owned(), 0, n)
    }

    fn from_buf(buf: Vec<u32>, head: usize, n: usize) -> Self {
        let mut blocked = vec![false; n];
        blocked[NOTHING as usize] = true;
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
    d: usize,
    queue: &mut Queue,
    set: &mut Vec<u32>,
    recipe: &RecipeSet,
    cb: &mut impl FnMut(usize, &Queue, &[u32]) -> bool,
) {
    if cb(d, queue, set) {
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
        enum_set_rec(d + 1, queue, set, recipe, cb);
        set.pop();
        queue.truncate(tail);
    }
    queue.head = head;
}

fn get_n(init: &[u32], recipe: &RecipeSet) -> usize {
    init.iter().fold(recipe.max_item, |acc, &u| acc.max(u)) as usize + 1
}

pub fn enum_set(
    init: &[u32],
    recipe: &RecipeSet,
    cb: &mut impl FnMut(usize, &Queue, &[u32]) -> bool,
) {
    let mut queue = Queue::new(get_n(init, recipe), init);
    if cb(0, &queue, init) {
        return;
    }
    queue.head = queue.tail();
    for &u in init {
        for &v in init {
            if let Some(w) = recipe.get(u, v) {
                queue.enqueue(w);
            }
        }
    }
    let mut set = init.to_owned();
    enum_set_rec(1, &mut queue, &mut set, recipe, cb);
}

fn collect_new_pairs_leaf(
    first: &[u32],
    second: &[u32],
    recipe: &RecipeSet,
    new_pairs: &mut HashSet<Pair>,
) {
    for &u in first {
        for &v in second {
            if !recipe.contains(u, v) {
                new_pairs.insert(Pair::new(u, v));
            }
        }
        if !recipe.contains(u, u) {
            new_pairs.insert(Pair::new(u, u));
        }
    }
}

fn to_vec(iter: impl IntoIterator<Item = Pair>) -> Vec<Pair> {
    let mut vec = Vec::from_iter(iter);
    vec.sort();
    vec
}

pub fn collect_new_pairs(depth: usize, init: &[u32], recipe: &RecipeSet) -> (Vec<Pair>, u64) {
    let pd = depth.min(5);
    let mut states = Vec::new();
    enum_set(init, recipe, &mut |d: usize, queue, set| {
        if d < pd {
            return false;
        }
        states.push((queue.buf.clone(), queue.head, set.to_owned()));
        true
    });

    let mut new_pairs_list = vec![HashSet::new(); states.len()];
    let count = states
        .into_par_iter()
        .zip(new_pairs_list.par_iter_mut())
        .map(|((buf, head, mut set), new_pairs)| {
            let mut queue = Queue::from_buf(buf, head, get_n(init, recipe));
            let mut count = 0u64;
            enum_set_rec(pd, &mut queue, &mut set, recipe, &mut |d, queue, set| {
                if d < depth {
                    return false;
                }
                collect_new_pairs_leaf(queue.as_slice(), set, recipe, new_pairs);
                count += 1;
                true
            });
            count
        })
        .sum::<u64>();

    let new_pairs: HashSet<Pair> = new_pairs_list.into_iter().flatten().collect();
    (to_vec(new_pairs), count)
}
