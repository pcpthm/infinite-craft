#![allow(clippy::new_without_default, clippy::len_without_is_empty)]

use std::collections::{HashMap, HashSet};

pub mod set_enum;
pub mod subset;

pub const UNKNOWN: u32 = 0;
pub const NOTHING: u32 = 1;
pub const PLACEHOLDER: u32 = !0;

pub struct ElementSet {
    name: Vec<String>,
    by_name: HashMap<String, u32>,
    token_count: HashMap<u32, usize>,
}

impl ElementSet {
    pub fn new() -> Self {
        Self {
            name: vec!["=unknown=".to_owned(), "Nothing".to_owned()],
            by_name: [("".to_owned(), NOTHING)].into_iter().collect(),
            token_count: HashMap::new(),
        }
    }

    pub fn intern(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.by_name.get(name) {
            return id;
        }
        let id = self.name.len() as u32;
        self.name.push(name.to_string());
        self.by_name.insert(name.to_owned(), id);
        id
    }

    #[inline]
    pub fn lookup(&self, name: &str) -> Option<u32> {
        self.by_name.get(name).copied()
    }

    #[inline]
    pub fn name(&self, id: u32) -> &str {
        &self.name[id as usize]
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.name.len()
    }

    #[inline]
    pub fn items(&self) -> impl Iterator<Item = u32> {
        NOTHING + 1..self.len() as u32
    }

    pub fn token_count(&self, u: u32) -> Option<usize> {
        self.token_count.get(&u).copied()
    }

    pub fn set_token_count(&mut self, u: u32, count: usize) {
        self.token_count.insert(u, count);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pair(u64);

impl Pair {
    #[inline]
    pub fn new(u: u32, v: u32) -> Self {
        Self((u as u64) << 32 | v as u64)
    }

    #[inline]
    pub fn get(self) -> [u32; 2] {
        [(self.0 >> 32) as u32, self.0 as u32]
    }
}

#[inline]
pub fn sym_pair(u: u32, v: u32) -> (Pair, bool) {
    let (u, v, b) = if u <= v { (u, v, false) } else { (v, u, true) };
    (Pair::new(u, v), b)
}

#[derive(Clone)]
pub struct RecipeSet(HashMap<Pair, [u32; 2]>);

impl RecipeSet {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    #[inline]
    pub fn get(&self, u: u32, v: u32) -> u32 {
        let (k, b) = sym_pair(u, v);
        self.0.get(&k).map(|r| r[b as usize]).unwrap_or(UNKNOWN)
    }

    #[inline]
    pub fn contains(&self, u: u32, v: u32) -> bool {
        let (k, b) = sym_pair(u, v);
        self.0.get(&k).is_some_and(|r| r[b as usize] != UNKNOWN)
    }

    #[inline]
    pub fn insert_half(&mut self, u: u32, v: u32, w: u32) {
        let (k, b) = sym_pair(u, v);
        self.0.entry(k).or_insert([UNKNOWN, UNKNOWN])[b as usize] = w;
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

pub fn get_path(init: &[u32], set: &[u32], recipe: &RecipeSet) -> Vec<[u32; 3]> {
    let mut queue = Vec::new();
    for (i, &u) in init.iter().enumerate() {
        for &v in init[..i + 1].iter().rev() {
            queue.push([u, v]);
        }
    }
    let mut qh = 0;
    let mut path = Vec::new();
    let mut set: HashSet<_> = set.iter().copied().collect();
    while !set.is_empty() && qh < queue.len() {
        let [u, v] = queue[qh];
        qh += 1;

        let w = recipe.get(u, v);
        if set.remove(&w) {
            path.push([u, v, w]);

            for &[_, _, x] in path.iter().rev() {
                queue.push([w, x]);
            }
            for &x in init.iter().rev() {
                queue.push([w, x]);
            }
        }
    }
    path
}
