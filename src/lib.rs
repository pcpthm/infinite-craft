pub mod search;
pub mod uniform_family;

use std::{collections::HashMap, fmt::Debug};

#[derive(Debug, Clone, Copy)]
pub struct Recipe {
    pub pair: [u32; 2],
    pub result: u32,
}

impl Recipe {
    pub fn new(first: u32, second: u32, result: u32) -> Self {
        Self {
            pair: [first, second],
            result,
        }
    }
}

pub struct RecipeMap {
    names: Vec<String>,
    id: HashMap<String, u32>,
    map: HashMap<[u32; 2], u32>,
}

impl RecipeMap {
    pub fn new() -> Self {
        Self {
            names: Vec::new(),
            id: HashMap::new(),
            map: HashMap::new(),
        }
    }

    #[inline]
    pub fn num_recipes(&self) -> usize {
        self.map.len()
    }

    #[inline]
    pub fn num_items(&self) -> usize {
        self.names.len()
    }

    #[inline]
    pub fn items(&self) -> impl Iterator<Item = u32> + DoubleEndedIterator<Item = u32> {
        0..self.num_items() as u32
    }

    #[inline]
    pub fn get(&self, first: u32, second: u32) -> Option<u32> {
        self.map.get(&[first, second]).copied()
    }

    pub fn insert(&mut self, first: &str, second: &str, result: &str) -> bool {
        let (first, second) = if first > second {
            (second, first)
        } else {
            (first, second)
        };
        let first = self.intern(first);
        let second = self.intern(second);

        if let Some(&existing) = self.map.get(&[first, second]) {
            if self.name(existing) != result {
                eprintln!(
                    "Not adding conflicting recipe {} + {} = {} vs existing {}",
                    self.name(first),
                    self.name(second),
                    self.name(existing),
                    result,
                );
            }
            return false;
        }

        let result = self.intern(result);
        self.map.insert([first, second], result);

        if first != second {
            self.map.insert([second, first], result);
        }
        true
    }

    #[inline]
    pub fn id(&self, name: &str) -> u32 {
        *self.id.get(name).expect("invalid item name")
    }

    #[inline]
    pub fn get_id(&self, name: &str) -> Option<u32> {
        self.id.get(name).copied()
    }

    fn intern(&mut self, name: &str) -> u32 {
        if let Some(id) = self.id.get(name).copied() {
            return id;
        }
        let id = u32::try_from(self.names.len()).expect("intern id overflow");
        self.id.insert(name.to_string(), id);
        self.names.push(name.to_string());
        id
    }

    pub fn name(&self, id: u32) -> &str {
        self.names.get(id as usize).expect("invalid item id")
    }

    pub fn make_graph(&self) -> Graph {
        Graph::from_recipe_map(self.num_items(), &self.map)
    }

    pub fn as_serializable(&self) -> (&[String], Vec<([u32; 2], u32)>) {
        let pairs = self.map.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>();
        (&self.names, pairs)
    }

    pub fn from_serialized((names, pairs): (Vec<&str>, Vec<([u32; 2], u32)>)) -> Self {
        Self {
            names: names.iter().map(|&name| name.to_owned()).collect(),
            id: names
                .iter()
                .zip(0..)
                .map(|(&name, u)| (name.to_owned(), u))
                .collect(),
            map: pairs.into_iter().collect(),
        }
    }
}

impl Default for RecipeMap {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Graph {
    start: Vec<usize>,
    arcs: Vec<(u32, u32)>,
    rev_start: Vec<usize>,
    rev_arcs: Vec<[u32; 2]>,
    map: HashMap<[u32; 2], u32>,
}

impl Graph {
    pub fn from_recipe_map(n: usize, map: &HashMap<[u32; 2], u32>) -> Self {
        let mut start = vec![0usize; n + 1];
        let mut rev_start = vec![0usize; n + 1];
        for (&[first, second], &result) in map {
            start[first as usize] += 1;

            if first <= second {
                rev_start[result as usize] += 1;
            }
        }
        for i in 0..n {
            start[i + 1] += start[i];
            rev_start[i + 1] += rev_start[i];
        }

        let mut arcs = vec![(0, 0); start[n]];
        let mut rev_arcs = vec![[0; 2]; rev_start[n]];
        for (&[first, second], &result) in map {
            start[first as usize] -= 1;
            arcs[start[first as usize]] = (second, result);

            if first <= second {
                rev_start[result as usize] -= 1;
                rev_arcs[rev_start[result as usize]] = [first, second];
            }
        }

        for i in 0..n {
            arcs[start[i]..start[i + 1]].sort();
            rev_arcs[rev_start[i]..rev_start[i + 1]].sort();
        }

        Self {
            start,
            arcs,
            rev_start,
            rev_arcs,
            map: map.clone(),
        }
    }

    #[inline]
    pub fn num_vertices(&self) -> usize {
        self.start.len() - 1
    }

    #[inline]
    pub fn arcs_from(&self, u: u32) -> impl Iterator<Item = (u32, u32)> + '_ {
        let end = self.start[u as usize + 1];
        let start = self.start[u as usize];
        self.arcs[start..end].iter().copied()
    }

    #[inline]
    pub fn arcs_to(&self, u: u32) -> impl Iterator<Item = [u32; 2]> + '_ {
        let end = self.rev_start[u as usize + 1];
        let start = self.rev_start[u as usize];
        self.rev_arcs[start..end].iter().copied()
    }

    #[inline]
    pub fn get_result(&self, first: u32, second: u32) -> Option<u32> {
        self.map.get(&[first, second]).copied()
    }
}

pub fn find_path(source: &[u32], set: &[u32], rm: &RecipeMap) -> Option<Vec<Recipe>> {
    let mut recipes: HashMap<u32, Option<Recipe>> = set.iter().map(|&u| (u, None)).collect();
    for (i, &u1) in source.iter().enumerate() {
        for &u2 in source[..i + 1].iter().rev() {
            find_path_try_add(u1, u2, &mut recipes, rm);
        }
    }

    let mut path: Vec<Recipe> = Vec::with_capacity(set.len());
    for _ in 0..set.len() {
        let recipe = set.iter().find_map(|&u3| *recipes.get(&u3)?)?;
        path.push(recipe);

        let u1 = recipe.result;
        recipes.remove(&u1);

        for u2 in path.iter().rev().map(|r| r.result) {
            find_path_try_add(u1, u2, &mut recipes, rm);
        }
        for &u2 in source.iter().rev() {
            find_path_try_add(u1, u2, &mut recipes, rm);
        }
    }
    Some(path)
}

fn find_path_try_add(u1: u32, u2: u32, recipes: &mut HashMap<u32, Option<Recipe>>, rm: &RecipeMap) {
    if let Some(u3) = rm.get(u1, u2) {
        if let Some(recipe) = recipes.get_mut(&u3) {
            if recipe.is_none() {
                *recipe = Some(Recipe::new(u1, u2, u3))
            }
        }
    }
}
