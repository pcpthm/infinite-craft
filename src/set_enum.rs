use std::collections::{HashMap, HashSet};

use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator,
};

pub const BLOCKED: u8 = 1;
pub const BLOCKED_FIRST: u8 = 2;
pub const BLOCKED_SECOND: u8 = 4;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymPair(u64);

impl SymPair {
    pub fn new(u: u32, v: u32) -> Self {
        let (u, v) = if u > v { (v, u) } else { (u, v) };
        Self((u as u64) << 32 | v as u64)
    }

    pub fn get(self) -> [u32; 2] {
        [(self.0 >> 32) as u32, self.0 as u32]
    }
}

fn enum_rec(
    d: usize,
    qh: usize,
    queue: &mut Vec<u32>,
    blocked: &mut Vec<u8>,
    set: &mut Vec<u32>,
    recipe: &HashMap<SymPair, u32>,
    on_leaf: &mut impl FnMut(usize, &[u32], &[u32], &[u8]),
) {
    if d == 0 {
        on_leaf(qh, queue, set, blocked);
        return;
    }
    let qt = queue.len();
    for i in qh..qt {
        let u = queue[i];
        if blocked[u as usize] & BLOCKED_SECOND == 0 {
            set.push(u);
        }
        if blocked[u as usize] & BLOCKED_FIRST == 0 {
            for &v in set.iter() {
                if let Some(&w) = recipe.get(&SymPair::new(u, v)) {
                    if blocked[w as usize] & BLOCKED == 0 {
                        blocked[w as usize] |= BLOCKED;
                        queue.push(w);
                    }
                }
            }
        }
        enum_rec(d - 1, i + 1, queue, blocked, set, recipe, on_leaf);
        for &w in &queue[qt..] {
            blocked[w as usize] &= !BLOCKED;
        }
        queue.truncate(qt);
        if set.last() == Some(&u) {
            set.pop();
        }
    }
}

pub fn collect_states(
    depth: usize,
    init: &[u32],
    blocked: &[u8],
    recipe: &HashMap<SymPair, u32>,
    states: &mut Vec<(usize, Vec<u32>, Vec<u32>)>,
) {
    let mut queue = Vec::new();
    let mut blocked = blocked.to_owned();
    let mut set = Vec::new();

    set.extend_from_slice(init);
    for &u in init {
        blocked[u as usize] |= BLOCKED | BLOCKED_SECOND;
    }

    for &u in init {
        queue.push(u);
        enum_rec(
            depth,
            0,
            &mut queue,
            &mut blocked,
            &mut set,
            recipe,
            &mut |qh, queue, set, _| {
                states.push((qh, queue.to_owned(), set.to_owned()));
            },
        );
        queue.pop();
    }
}

pub fn collect_new_pairs(
    depth: usize,
    init: &[u32],
    blocked: &[u8],
    recipe: &HashMap<SymPair, u32>,
    new_pairs: &mut HashSet<SymPair>,
) {
    let parallel_depth = (depth - 1).min(5);
    let mut states = Vec::new();
    collect_states(parallel_depth, init, blocked, recipe, &mut states);

    let mut new_pairs_list = vec![HashSet::new(); states.len()];
    states
        .into_par_iter()
        .zip(new_pairs_list.par_iter_mut())
        .for_each(|((qh, mut queue, mut set), new_pairs)| {
            let mut blocked = blocked.to_owned();
            for &u in queue.iter() {
                blocked[u as usize] |= BLOCKED;
            }
            enum_rec(
                depth - 1 - parallel_depth,
                qh,
                &mut queue,
                &mut blocked,
                &mut set,
                recipe,
                &mut |qh, queue, set, blocked| {
                    for &u in &queue[qh..] {
                        if blocked[u as usize] & BLOCKED_FIRST == 0 {
                            for &v in set.iter() {
                                if !recipe.contains_key(&SymPair::new(u, v)) {
                                    new_pairs.insert(SymPair::new(u, v));
                                }
                            }
                            if blocked[u as usize] & BLOCKED_SECOND == 0
                                && !recipe.contains_key(&SymPair::new(u, u))
                            {
                                new_pairs.insert(SymPair::new(u, u));
                            }
                        }
                    }
                },
            );
        });
    new_pairs.extend(new_pairs_list.into_iter().flatten());
}
