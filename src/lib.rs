#![allow(clippy::new_without_default, clippy::len_without_is_empty)]

use std::collections::{HashMap, HashSet};

pub mod set_enum;
pub mod subset;

pub const NOTHING: u32 = 0;

pub struct ElementSet {
    name: Vec<String>,
    by_name: HashMap<String, u32>,
    by_lcname: HashMap<String, u32>,
    canon: Vec<u32>,
    token_count: HashMap<u32, usize>,
}

impl ElementSet {
    pub fn new() -> Self {
        Self {
            name: vec!["Nothing".to_owned()],
            by_name: [("".to_owned(), NOTHING)].into_iter().collect(),
            by_lcname: HashMap::new(),
            canon: vec![NOTHING],
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

        let lcname = name.to_lowercase();
        self.canon.push(*self.by_lcname.entry(lcname).or_insert(id));

        id
    }

    #[inline]
    pub fn name(&self, id: u32) -> &str {
        &self.name[id as usize]
    }

    #[inline]
    pub fn canon(&self, id: u32) -> u32 {
        self.canon[id as usize]
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
    pub fn get(&self) -> [u32; 2] {
        [(self.0 >> 32) as u32, self.0 as u32]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymPair(u64);

impl SymPair {
    #[inline]
    pub fn new(u: u32, v: u32) -> Self {
        let (u, v) = if u >= v { (u, v) } else { (v, u) };
        Self((u as u64) << 32 | v as u64)
    }

    #[inline]
    pub fn get(&self) -> [u32; 2] {
        [(self.0 >> 32) as u32, self.0 as u32]
    }
}

pub fn get_path(init: &[u32], set: &[u32], recipe: &HashMap<Pair, u32>) -> Vec<[u32; 3]> {
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

        if let Some(&w) = recipe.get(&Pair::new(u, v)) {
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
    }
    path
}
