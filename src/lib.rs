#![allow(clippy::new_without_default, clippy::len_without_is_empty)]

use std::collections::{HashMap, HashSet};

pub mod set_enum;
// pub mod subset;

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

pub const NOTHING: u32 = 0;

pub struct NameMap {
    name: Vec<String>,
    id: HashMap<String, u32>,
}

impl NameMap {
    pub fn new() -> Self {
        Self {
            name: vec!["Nothing".to_owned()],
            id: [("".to_owned(), NOTHING)].into_iter().collect(),
        }
    }

    pub fn intern(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.id.get(name) {
            return id;
        }
        let id = self.name.len() as u32;
        self.name.push(name.to_string());
        self.id.insert(name.to_owned(), id);
        id
    }

    #[inline]
    pub fn lookup(&self, name: &str) -> Option<u32> {
        self.id.get(name).copied()
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
        1u32..self.len() as u32
    }
}

pub struct Recipe(HashMap<SymPair, u32>);

impl Recipe {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    #[inline]
    pub fn get(&self, u: u32, v: u32) -> Option<u32> {
        self.0.get(&SymPair::new(u, v)).copied()
    }

    #[inline]
    pub fn contains(&self, u: u32, v: u32) -> bool {
        self.0.contains_key(&SymPair::new(u, v))
    }

    #[inline]
    pub fn insert(&mut self, u: u32, v: u32, w: u32) {
        self.0.insert(SymPair::new(u, v), w);
    }
}

pub fn get_path(init: &[u32], set: &[u32], recipe: &Recipe) -> Vec<[u32; 3]> {
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

        if let Some(w) = recipe.get(u, v) {
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
