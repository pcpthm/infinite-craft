use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    mem::{replace, take},
};

use crate::{uniform_family::UniformFamily, Graph};

pub struct Search {
    max_card: usize,
    max_count: usize,
    queue: BinaryHeap<(Reverse<usize>, u32)>,
    in_que: Vec<usize>,
    sets: Vec<UniformFamily>,
}

impl Search {
    pub fn new(num_items: usize, max_count: usize, max_card: usize) -> Self {
        Self {
            max_card,
            max_count,
            queue: BinaryHeap::new(),
            in_que: vec![usize::MAX; num_items],
            sets: vec![UniformFamily::new(); num_items],
        }
    }

    #[inline]
    pub fn reached(&self, u: u32) -> bool {
        !self.sets[u as usize].is_empty()
    }

    #[inline]
    pub fn sets(&self, u: u32) -> &UniformFamily {
        &self.sets[u as usize]
    }

    #[inline]
    pub fn min_card(&self, u: u32) -> usize {
        self.sets(u).card()
    }

    pub fn search_from_source(&mut self, source: &[u32], graph: &Graph) {
        for &u in source {
            self.sets[u as usize].set_single_empty();
            self.push(u, 0);
        }
        self.search(graph);
    }

    fn search(&mut self, graph: &Graph) {
        while let Some((Reverse(c), u)) = self.queue.pop() {
            if c == replace(&mut self.in_que[u as usize], usize::MAX) {
                self.relax_from(u, graph);
            }
        }
    }

    fn relax_from(&mut self, u1: u32, graph: &Graph) {
        let c1 = self.min_card(u1);
        for (u2, u3) in graph.arcs_from(u1) {
            if c1 < self.min_card(u2) || self.min_card(u3) <= c1 {
                continue;
            }
            let mut sets3 = take(&mut self.sets[u3 as usize]);
            if sets3.add_merge(self.sets(u1), self.sets(u2), u3, self.max_count) {
                self.push(u3, sets3.card());
            }
            self.sets[u3 as usize] = sets3;
        }
    }

    fn push(&mut self, u3: u32, c3: usize) {
        if c3 <= self.max_card && c3 < self.in_que[u3 as usize] {
            self.in_que[u3 as usize] = c3;
            self.queue.push((Reverse(c3), u3));
        }
    }
}

#[cfg(test)]
mod test {
    use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};

    use crate::{find_path, RecipeMap};

    use super::*;

    #[test]
    fn search_with_real_rm() -> Result<(), Box<dyn std::error::Error>> {
        let rm = RecipeMap::from_serialized(bincode::deserialize(&std::fs::read(
            "recipe-map.bincode",
        )?)?);
        let source = ["Water", "Fire", "Wind", "Earth"].map(|u| rm.id(u));
        let max_card = 300;
        let card1 = {
            let mut search = Search::new(rm.num_items(), usize::MAX, max_card);
            search.search_from_source(&source, &rm.make_graph());
            check_all_paths(&search, &source, &rm);
            get_card_all(&search)
        };

        // should give a consistent result regardless of the order of the recipes
        let mut rng = StdRng::seed_from_u64(1);
        let mut graph = rm.make_graph();
        for i in 0..graph.num_vertices() {
            graph.arcs[graph.start[i]..graph.start[i + 1]].shuffle(&mut rng);
        }
        let card2 = {
            let mut search = Search::new(rm.num_items(), usize::MAX, max_card);
            search.search_from_source(&source, &rm.make_graph());
            get_card_all(&search)
        };
        for u in rm.items() {
            assert_eq!(card1[u as usize], card2[u as usize], "{:?}", rm.name(u));
        }

        Ok(())
    }

    fn check_all_paths(search: &Search, source: &[u32], rm: &RecipeMap) {
        for u in rm.items() {
            for set in search.sets(u).iter() {
                let path = find_path(&source, set, &rm).unwrap();
                let mut path_set: Vec<_> = path.iter().map(|r| r.result).collect();
                path_set.sort();
                assert_eq!(&path_set, set);
            }
        }
    }

    fn get_card_all(search: &Search) -> Vec<usize> {
        search.sets.iter().map(|sets| sets.card()).collect()
    }
}
