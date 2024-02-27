//! Recipe cache.

use std::collections::HashMap;

pub const NOTHING: u32 = 0;
const NONE: usize = !0;

pub struct Cache {
    id: HashMap<String, u32>,
    name: Vec<String>,
    token: Vec<usize>,
    pair: HashMap<[u32; 2], u32>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            id: [("".to_string(), 0)].into_iter().collect(),
            name: vec!["Nothing".to_string()],
            token: vec![0],
            pair: Default::default(),
        }
    }

    pub fn intern(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.id.get(name) {
            return id;
        }
        let id = self.name.len() as u32;
        self.id.insert(name.to_string(), id);
        self.name.push(name.to_string());
        self.token.push(NONE);
        id
    }

    pub fn name(&self, id: u32) -> &str {
        self.name[id as usize].as_str()
    }

    pub fn insert_pair(&mut self, pair: [u32; 2], result: u32) {
        assert!(pair[0] != NOTHING && pair[1] != NOTHING);

        self.pair.insert(pair, result);
        if pair[0] != pair[1] {
            self.pair.insert([pair[1], pair[0]], result);
        }
    }

    pub fn pair(&self, pair: [u32; 2]) -> Option<u32> {
        self.pair.get(&pair).copied()
    }

    pub fn insert_tokenize(&mut self, item: u32, count: usize) {
        self.token[item as usize] = count;
    }

    pub fn tokenize(&self, id: u32) -> Option<usize> {
        Some(self.token[id as usize]).filter(|&c| c != NONE)
    }
}
