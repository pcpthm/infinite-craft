use std::collections::HashMap;

use crate::Pair;

fn closure(queue: &mut Vec<u32>, remain: &mut [bool], recipe: &HashMap<Pair, u32>) {
    let mut qh = 0;
    while qh < queue.len() {
        let u = queue[qh];
        qh += 1;
        for i in 0..qh {
            if let Some(&w) = recipe.get(&Pair::new(u, queue[i])) {
                if std::mem::replace(&mut remain[w as usize], false) {
                    queue.push(w);
                }
            }
        }
    }
}

fn is_reachable(
    target: &[u32],
    queue: &mut Vec<u32>,
    remain: &mut [bool],
    recipe: &HashMap<Pair, u32>,
) -> bool {
    let head = queue.len();
    closure(queue, remain, recipe);
    let reachable = target.iter().all(|&u| !remain[u as usize]);
    for &u in &queue[head..] {
        remain[u as usize] = true;
    }
    queue.truncate(head);
    reachable
}

pub fn get_unreachable(
    init: &[u32],
    extra: &[u32],
    target: &[u32],
    remain: &mut [bool],
    recipe: &HashMap<Pair, u32>,
) -> Vec<u32> {
    let mut queue = init.to_owned();
    for &u in target.iter().chain(extra) {
        remain[u as usize] = true;
    }
    closure(&mut queue, remain, recipe);
    let res = Vec::from_iter(target.iter().copied().filter(|&u| remain[u as usize]));
    for &u in target.iter().chain(extra) {
        remain[u as usize] = false;
    }
    res
}

fn dfs(
    target: &[u32],
    extra: &[u32],
    queue: &mut Vec<u32>,
    remain: &mut [bool],
    removed: &mut Vec<u32>,
    recipe: &HashMap<Pair, u32>,
    out: &mut Vec<Vec<u32>>,
) {
    let mut right_maximal = true;
    for i in 0..extra.len() {
        let u = extra[i];
        assert!(remain[u as usize]);
        remain[u as usize] = false;
        if is_reachable(target, queue, remain, recipe) {
            right_maximal = false;
            removed.push(u);
            dfs(target, &extra[i + 1..], queue, remain, removed, recipe, out);
            removed.pop();
        }
        remain[u as usize] = true;
    }
    if right_maximal {
        if out.is_empty() || out[0].len() < removed.len() {
            out.clear();
        }
        if out.is_empty() || out[0].len() == removed.len() {
            out.push(removed.clone());
        }
    }
}

pub fn get_max_removal(
    init: &[u32],
    target: &[u32],
    extra: &[u32],
    remain: &mut [bool],
    recipe: &HashMap<Pair, u32>,
    out: &mut Vec<Vec<u32>>,
) {
    let mut queue = init.to_owned();
    for &u in target.iter().chain(extra) {
        remain[u as usize] = true;
    }
    let mut removed = Vec::new();
    dfs(target, extra, &mut queue, remain, &mut removed, recipe, out);
    for &u in target.iter().chain(extra) {
        remain[u as usize] = false;
    }
}
