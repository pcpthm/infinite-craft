use std::collections::HashSet;

use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator,
};

use crate::{Recipe, SymPair};

pub const BLOCKED: u8 = 1;
pub const BLOCKED_FIRST: u8 = 2;
pub const BLOCKED_SECOND: u8 = 4;

fn enum_set_rec(
    d: usize,
    qh: usize,
    queue: &mut Vec<u32>,
    blocked: &mut Vec<u8>,
    set: &mut Vec<u32>,
    recipe: &Recipe,
    cb: &mut impl FnMut(usize, &[u32], &[u32], &[u8]),
) {
    if d == 0 {
        cb(qh, queue, set, blocked);
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
                if let Some(w) = recipe.get(u, v) {
                    if blocked[w as usize] & BLOCKED == 0 {
                        blocked[w as usize] |= BLOCKED;
                        queue.push(w);
                    }
                }
            }
        }
        enum_set_rec(d - 1, i + 1, queue, blocked, set, recipe, cb);
        for &w in &queue[qt..] {
            blocked[w as usize] &= !BLOCKED;
        }
        queue.truncate(qt);
        if set.last() == Some(&u) {
            set.pop();
        }
    }
}

pub fn enum_set(
    d: usize,
    init: &[u32],
    blocked: &[u8],
    recipe: &Recipe,
    cb: &mut impl FnMut(usize, &[u32], &[u32], &[u8]),
) {
    assert!(d > 0);
    let mut queue = Vec::new();
    let mut blocked = blocked.to_owned();
    let mut set = Vec::new();
    for &u in init {
        if blocked[u as usize] & BLOCKED_SECOND == 0 {
            set.push(u);
        }
        if blocked[u as usize] & BLOCKED_FIRST == 0 {
            for &v in &set {
                if let Some(w) = recipe.get(u, v) {
                    if blocked[w as usize] & BLOCKED == 0 {
                        blocked[w as usize] |= BLOCKED;
                        queue.push(w);
                    }
                }
            }
        }
    }
    enum_set_rec(d - 1, 0, &mut queue, &mut blocked, &mut set, recipe, cb);
}

fn collect_new_pairs_leaf(
    qh: usize,
    queue: &[u32],
    blocked: &[u8],
    set: &[u32],
    recipe: &Recipe,
    new_pairs: &mut HashSet<SymPair>,
) {
    for &u in &queue[qh..] {
        if blocked[u as usize] & BLOCKED_FIRST == 0 {
            for &v in set.iter() {
                if !recipe.contains(u, v) {
                    new_pairs.insert(SymPair::new(u, v));
                }
            }
            if blocked[u as usize] & BLOCKED_SECOND == 0 && !recipe.contains(u, u) {
                new_pairs.insert(SymPair::new(u, u));
            }
        }
    }
}

pub fn collect_new_pairs(
    depth: usize,
    init: &[u32],
    blocked: &[u8],
    recipe: &Recipe,
    new_pairs: &mut HashSet<SymPair>,
) {
    let pd = depth.min(5);
    let mut states = Vec::new();
    if pd == 0 {
        for &u in init {
            if blocked[u as usize] & BLOCKED_SECOND == 0 {
                states.push((0, vec![u], init.to_owned()));
            }
        }
    } else {
        enum_set(pd, init, blocked, recipe, &mut |qh, queue, set, _| {
            states.push((qh, queue.to_owned(), set.to_owned()));
        });
    }

    let mut new_pairs_list = vec![HashSet::new(); states.len()];
    states
        .into_par_iter()
        .zip(new_pairs_list.par_iter_mut())
        .for_each(|((qh, mut queue, mut set), new_pairs)| {
            let mut blocked = blocked.to_owned();
            for &u in queue.iter() {
                blocked[u as usize] |= BLOCKED;
            }
            enum_set_rec(
                depth - pd,
                qh,
                &mut queue,
                &mut blocked,
                &mut set,
                recipe,
                &mut |qh, queue, set, blocked| {
                    collect_new_pairs_leaf(qh, queue, blocked, set, recipe, new_pairs);
                },
            );
        });
    new_pairs.extend(new_pairs_list.into_iter().flatten());
}
