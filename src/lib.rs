use std::{
    fmt::{Debug, Display},
    hash::Hash,
    mem::replace,
    ops::{Index, IndexMut},
};

pub type Generation = u32;

#[derive(Clone, Debug)]
pub struct Arena<T> {
    entries: Vec<Entry<T>>,
    free_list_head: Option<usize>,
    length: usize,
}
impl<T> Arena<T> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            free_list_head: None,
            length: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.length
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, t: T) -> ID {
        self.insert_with_id(|_| t)
    }
    pub fn insert_with_id<F>(&mut self, f: F) -> ID
    where
        F: FnOnce(ID) -> T,
    {
        let id = if let Some(free) = self.free_list_head.take() {
            let &Entry::Free { next_generation, next_free } = &self.entries[free] else { unreachable!() };
            self.free_list_head = next_free;

            ID::new(free, next_generation)
        } else {
            let index = self.entries.len();
            self.entries.push(Entry::Free {
                next_generation: 0,
                next_free: None,
            });
            ID::new(index, 0)
        };
        self.length += 1;

        let item = f(id);
        self.entries[id.index] = Entry::Occupied(id.generation, item);

        id
    }
    pub fn remove(&mut self, id: ID) -> Option<T> {
        if !self.contains(id) {
            return None;
        }

        let new_entry = Entry::Free {
            next_free: self.free_list_head,
            next_generation: id.generation + 1,
        };
        let old_entry = std::mem::replace(&mut self.entries[id.index], new_entry);

        self.free_list_head = Some(id.index);

        let Entry::Occupied(_, item) = old_entry else { unreachable!() };
        self.length -= 1;
        Some(item)
    }

    pub fn get(&self, id: ID) -> Option<&T> {
        let Some(entry) = self.entries.get(id.index) else { return None };
        let Entry::Occupied(gen, item) = entry else { return None };

        if id.generation != *gen {
            None
        } else {
            Some(item)
        }
    }
    pub fn get_mut(&mut self, id: ID) -> Option<&mut T> {
        let Some(entry) = self.entries.get_mut(id.index) else { return None };
        let Entry::Occupied(gen, item) = entry else { return None };

        if id.generation != *gen {
            None
        } else {
            Some(item)
        }
    }
    pub fn contains(&self, id: ID) -> bool {
        self.get(id).is_some()
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            entries: &self.entries,
            index: 0,
        }
    }
    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            entries: &mut self.entries,
            index: 0,
        }
    }
    pub fn indices(&self) -> Indices<T> {
        let items = self.iter();
        Indices { items }
    }
}
impl<T> Index<ID> for Arena<T> {
    type Output = T;
    fn index(&self, index: ID) -> &Self::Output {
        self.get(index).unwrap()
    }
}
impl<T> IndexMut<ID> for Arena<T> {
    fn index_mut(&mut self, index: ID) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}
impl<T> IntoIterator for Arena<T> {
    type IntoIter = IntoIter<T>;
    type Item = T;
    fn into_iter(self) -> Self::IntoIter {
        let entries = self.entries.into_iter();
        IntoIter { entries }
    }
}
impl<'a, T> IntoIterator for &'a Arena<T> {
    type IntoIter = Iter<'a, T>;
    type Item = (ID, &'a T);
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, T> IntoIterator for &'a mut Arena<T> {
    type IntoIter = IterMut<'a, T>;
    type Item = (ID, &'a mut T);
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[derive(Copy, Clone, Debug)]
enum Entry<T> {
    Free {
        next_generation: Generation,
        next_free: Option<usize>,
    },
    Occupied(Generation, T),
}

pub struct Iter<'a, T> {
    entries: &'a [Entry<T>],
    index: usize,
}
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (ID, &'a T);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match replace(&mut self.entries, &[]) {
                [] => return None,
                [first, rest @ ..] => {
                    self.entries = rest;
                    let index = self.index;
                    self.index += 1;

                    if let Entry::Occupied(gen, t) = first {
                        let id = ID::new(index, *gen);
                        return Some((id, t));
                    }
                }
            }
        }
    }
}
impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            let entries = replace(&mut self.entries, &[]);
            let (last, others) = entries.split_last()?;
            let index = self.index + others.len();
            self.entries = others;

            if let Entry::Occupied(gen, t) = last {
                let id = ID::new(index, *gen);
                return Some((id, t));
            }
        }
    }
}

pub struct IterMut<'a, T> {
    entries: &'a mut [Entry<T>],
    index: usize,
}
impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (ID, &'a mut T);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match replace(&mut self.entries, &mut []) {
                [] => return None,
                [first, rest @ ..] => {
                    self.entries = rest;
                    let index = self.index;
                    self.index += 1;

                    if let Entry::Occupied(gen, t) = first {
                        let id = ID::new(index, *gen);
                        return Some((id, t));
                    }
                }
            }
        }
    }
}
impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            let entries = replace(&mut self.entries, &mut []);
            let (last, others) = entries.split_last_mut()?;
            let index = self.index + others.len();
            self.entries = others;

            if let Entry::Occupied(gen, t) = last {
                let id = ID::new(index, *gen);
                return Some((id, t));
            }
        }
    }
}

pub struct IntoIter<T> {
    entries: std::vec::IntoIter<Entry<T>>,
}
impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry = self.entries.next()?;
            if let Entry::Occupied(_, t) = entry {
                return Some(t);
            }
        }
    }
}
impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            let entry = self.entries.next_back()?;
            if let Entry::Occupied(_, t) = entry {
                return Some(t);
            }
        }
    }
}

pub struct Indices<'a, T> {
    items: Iter<'a, T>,
}
impl<'a, T> Iterator for Indices<'a, T> {
    type Item = ID;
    fn next(&mut self) -> Option<Self::Item> {
        self.items.next().map(|(i, _)| i)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ID {
    index: usize,
    generation: Generation,
}
impl ID {
    fn new(index: usize, generation: Generation) -> Self {
        Self { index, generation }
    }

    pub fn index(&self) -> usize {
        self.index
    }
    pub fn generation(&self) -> Generation {
        self.generation
    }
}
impl Display for ID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.generation == 0 {
            write!(f, "{}", self.index)
        } else {
            write!(f, "({}-{})", self.index, self.generation)
        }
    }
}
