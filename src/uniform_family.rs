use std::{collections::BTreeSet, fmt::Debug};

#[derive(Debug, Clone)]
pub struct UniformFamily {
    card: usize,
    sets: BTreeSet<Vec<u32>>,
}

impl UniformFamily {
    pub fn new() -> Self {
        Self {
            card: usize::MAX,
            sets: BTreeSet::new(),
        }
    }

    pub fn card(&self) -> usize {
        self.card
    }

    pub fn clear(&mut self) {
        self.card = usize::MAX;
        self.sets.clear();
    }

    pub fn set_single_empty(&mut self) {
        self.card = 0;
        self.sets.clear();
        self.sets.insert(vec![]);
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.card == usize::MAX
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.sets.len()
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &[u32]> {
        self.sets.iter().map(|set| set.as_slice())
    }

    pub fn add_merge(&mut self, sets1: &Self, sets2: &Self, u3: u32, max_len: usize) -> bool {
        let mut updated = false;
        for set1 in sets1.iter() {
            updated |= self.add_merge_1(set1, sets2, u3, max_len);
        }
        updated
    }

    pub fn add_merge_1(&mut self, set1: &[u32], sets2: &Self, u3: u32, max_len: usize) -> bool {
        let mut updated = false;
        let u3_in_set1 = set1.contains(&u3);
        for set2 in sets2.iter() {
            let c3 = merge_card(set1, set2) + !(u3_in_set1 || set2.contains(&u3)) as usize;
            if self.card < c3 {
                continue;
            }
            if c3 < self.card {
                self.card = c3;
                self.sets.clear();
                updated = true;
            }
            if max_len <= self.len() {
                continue;
            }

            let mut set3 = Vec::with_capacity(c3);
            merge(set1, set2, &mut set3);
            if let Err(i) = set3.binary_search(&u3) {
                set3.insert(i, u3);
            }

            assert!(set3.len() == c3);

            self.sets.insert(set3);
        }
        updated
    }
}

impl Default for UniformFamily {
    fn default() -> Self {
        Self::new()
    }
}

fn merge_card(set1: &[u32], set2: &[u32]) -> usize {
    let mut iter2 = set2.iter().peekable();
    let mut card3 = set1.len() + set2.len();
    for &u1 in set1 {
        while let Some(&u2) = iter2.next_if(|&&u2| u2 <= u1) {
            card3 -= (u1 == u2) as usize;
        }
    }
    card3
}

fn merge(set1: &[u32], set2: &[u32], out: &mut Vec<u32>) {
    let mut iter2 = set2.iter().peekable();
    for &u1 in set1 {
        while let Some(&u2) = iter2.next_if(|&&u2| u2 <= u1) {
            if u1 != u2 {
                out.push(u2);
            }
        }
        out.push(u1);
    }
    out.extend(iter2);
}
