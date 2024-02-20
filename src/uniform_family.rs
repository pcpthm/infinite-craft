use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct UniformFamily {
    pub(crate) card: usize,
    pub(crate) buf: Vec<u32>, // store all sets in a continuous buffer, by chunks
}

impl UniformFamily {
    pub fn new() -> Self {
        Self {
            card: usize::MAX,
            buf: Vec::new(),
        }
    }

    pub fn card(&self) -> usize {
        self.card
    }

    pub fn clear(&mut self) {
        self.card = usize::MAX;
        self.buf.clear();
    }

    pub fn set_single_empty(&mut self) {
        self.card = 0;
        self.buf.clear();
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.card == usize::MAX
    }

    #[inline]
    pub fn len(&self) -> usize {
        if self.card == 0 {
            1
        } else {
            self.buf.len() / self.card
        }
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &[u32]> {
        (0..self.len()).map(|i| &self.buf[i * self.card..][..self.card])
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
                self.buf.clear();
                updated = true;
            }
            if max_len <= self.len() {
                continue;
            }

            let end_i = self.buf.len();
            merge_ex(set1, set2, u3, &mut self.buf);

            assert!(self.buf.len() == end_i + c3);
            if end_i == 0 {
                updated = true;
                continue;
            }

            let mut insert_i = end_i;
            while c3 <= insert_i && self.buf[end_i..] < self.buf[insert_i - c3..][..c3] {
                insert_i -= c3;
            }
            if c3 <= insert_i && self.buf[insert_i - c3..][..c3] == self.buf[end_i..] {
                self.buf.truncate(end_i);
            } else {
                self.buf[insert_i..].rotate_right(c3);
                updated = true;
            }
        }
        updated
    }

    pub fn sort_dedup(&mut self, buf: &mut Vec<u32>, tmp_i: &mut Vec<usize>) {
        if self.buf.len() <= self.card {
            return;
        }

        buf.clear();
        buf.extend_from_slice(&self.buf);

        let card = self.card;
        tmp_i.clear();
        tmp_i.extend((0..buf.len()).step_by(card));
        tmp_i.sort_by(|&i, &j| buf[i..][..card].cmp(&buf[j..][..card]));
        tmp_i.dedup_by(|&mut i, &mut j| buf[i..][..card] == buf[j..][..card]);

        self.buf.clear();
        for &i in tmp_i.iter() {
            self.buf.extend_from_slice(&buf[i..][..card]);
        }
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

fn merge_ex(set1: &[u32], set2: &[u32], u3: u32, out: &mut Vec<u32>) {
    let i_start = out.len();
    merge(set1, set2, out);

    let mut i_insert = out.len();
    while i_start < i_insert && u3 < out[i_insert - 1] {
        i_insert -= 1;
    }
    if !(i_start < i_insert && u3 == out[i_insert - 1]) {
        out.insert(i_insert, u3);
    }
}
