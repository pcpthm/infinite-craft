use std::collections::HashMap;

pub const NOTHING: u32 = 0;

pub struct PairCache {
    name: Vec<String>,
    id: HashMap<String, u32>,
    pair: HashMap<[u32; 2], u32>,
}

impl PairCache {
    pub fn new() -> Self {
        Self {
            name: vec!["Nothing".to_string()],
            id: [("".to_string(), 0)].into_iter().collect(),
            pair: Default::default(),
        }
    }

    pub fn num_items(&self) -> usize {
        self.name.len()
    }

    pub fn intern(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.id.get(name) {
            return id;
        }
        let id: u32 = self.name.len() as u32;
        self.name.push(name.to_string());
        self.id.insert(name.to_string(), id);
        id
    }

    pub fn name(&self, id: u32) -> &str {
        self.name[id as usize].as_str()
    }

    pub fn insert(&mut self, pair: [u32; 2], result: u32) {
        assert!(pair[0] != NOTHING && pair[1] != NOTHING);
        self.pair.insert(pair, result);
        if pair[0] != pair[1] {
            self.pair.insert([pair[1], pair[0]], result);
        }
    }

    pub fn get(&self, pair: [u32; 2]) -> Option<u32> {
        self.pair.get(&pair).copied()
    }
}
