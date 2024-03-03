use crate::RecipeSet;

const UNVISITED: u8 = 1;

fn closure(queue: &mut Vec<u32>, state: &mut [u8], recipe: &RecipeSet) {
    let mut qh = 0;
    while qh < queue.len() {
        let u = queue[qh];
        qh += 1;
        for i in 0..qh {
            let w = recipe.get(u, queue[i]);
            if state[w as usize] & UNVISITED != 0 {
                state[w as usize] &= !UNVISITED;
                queue.push(w);
            }
        }
    }
}

fn is_reachable(
    target: &[u32],
    queue: &mut Vec<u32>,
    state: &mut [u8],
    recipe: &RecipeSet,
) -> bool {
    let head = queue.len();
    closure(queue, state, recipe);
    let reachable = target.iter().all(|&u| state[u as usize] & UNVISITED == 0);
    for &u in &queue[head..] {
        state[u as usize] |= UNVISITED;
    }
    queue.truncate(head);
    reachable
}

pub fn get_unreachable(
    init: &[u32],
    extra: &[u32],
    target: &[u32],
    state: &mut [u8],
    recipe: &RecipeSet,
) -> Vec<u32> {
    let mut queue = init.to_owned();
    for &u in target.iter().chain(extra) {
        state[u as usize] |= UNVISITED;
    }
    closure(&mut queue, state, recipe);
    let mut res = Vec::new();
    for &u in target {
        if state[u as usize] & UNVISITED != 0 {
            res.push(u);
        }
    }
    for &u in target.iter().chain(extra) {
        state[u as usize] &= !UNVISITED;
    }
    res
}

fn dfs(
    target: &[u32],
    extra: &[u32],
    queue: &mut Vec<u32>,
    state: &mut [u8],
    removed: &mut Vec<u32>,
    recipe: &RecipeSet,
    sets: &mut Vec<Vec<u32>>,
) {
    let mut right_maximal = true;
    for i in 0..extra.len() {
        let u = extra[i];
        assert!(state[u as usize] & UNVISITED != 0);

        state[u as usize] &= !UNVISITED;
        if is_reachable(target, queue, state, recipe) {
            right_maximal = false;
            removed.push(u);
            dfs(target, &extra[i + 1..], queue, state, removed, recipe, sets);
            removed.pop();
        }
        state[u as usize] |= UNVISITED;
    }
    if right_maximal {
        if sets.is_empty() || sets[0].len() < removed.len() {
            sets.clear();
        }
        if sets.is_empty() || sets[0].len() == removed.len() {
            sets.push(removed.clone());
        }
    }
}

pub fn get_max_removal(
    init: &[u32],
    target: &[u32],
    extra: &[u32],
    state: &mut [u8],
    recipe: &RecipeSet,
    sets: &mut Vec<Vec<u32>>,
) {
    let mut queue = init.to_owned();
    for &u in target.iter().chain(extra) {
        state[u as usize] |= UNVISITED;
    }
    let mut removed = Vec::new();
    dfs(target, extra, &mut queue, state, &mut removed, recipe, sets);
    for &u in target.iter().chain(extra) {
        state[u as usize] &= !UNVISITED;
    }
}
