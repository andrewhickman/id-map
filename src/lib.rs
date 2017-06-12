//! A container that gives each item a unique id. Adding and removing by index is O(1).

#[cfg(test)] mod tests;

use std::iter::FromIterator;
use std::ops::{Index, IndexMut};
use std::{fmt, ptr};

use bit_set::{self, BitSet};
use bit_vec::BitVec;

#[derive(Clone, Copy, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
/// An identifier for an `IdMap` value.
pub struct Id(usize);

/// A container that gives each item a unique id.
pub struct IdMap<T> {
    // The set of valid indices for values.
    ids: BitSet,
    // The buffer of values. Indices not in ids are invalid.
    values: Vec<T>,
    // The index to start counting from when inserting.
    start: usize,
}

impl<T> IdMap<T> {
    /// Create an empty map.
    pub fn new() -> Self {
        IdMap {
            ids: BitSet::new(),
            values: Vec::new(),
            start: 0,
        }
    }

    /// Insert an value into the map and return its id.    
    pub fn insert(&mut self, val: T) -> Id {
        let end = self.values.len();
        for id in self.start..end {
            if self.ids.insert(id) {
                // The value in the vec is uninitialized.
                unsafe {
                    ptr::write(self.values.get_unchecked_mut(id), val);
                }
                self.start = id;
                return Id(id);
            }
        }
        self.ids.insert(end);
        self.values.push(val);
        self.start = end;
        Id(end)
    }

    /// Remove the value at the given index, returning it if it exists. 
    pub fn remove(&mut self, Id(id): Id) -> Option<T> {
        if self.ids.remove(id) {
            if id < self.start {
                self.start = id;
            }
            Some(if id + 1 == self.values.len() {
                // Note len() cannot be 0 here so unwrap cannot fail.
                self.values.pop().unwrap()
            } else {
                unsafe { ptr::read(self.values.get_unchecked(id)) }
            })
        } else {
            None
        }
    }

    /// Get an iterator over ids.
    pub fn ids(&self) -> Ids {
        Ids { ids: self.ids.iter(), start: self.start }
    }

    /// Get an iterator over values.
    pub fn values(&self) -> Values<T> {
        Values {
            ids: self.ids.iter(),
            values: &self.values,
            start: self.start,
        }
    }

    /// Get an iterator over id, value pairs.
    pub fn iter(&self) -> Iter<T> {
        Iter {
            ids: self.ids.iter(),
            values: &self.values,
            start: self.start,
        }
    }

    /// Check whether a given id has an entry in the map.
    pub fn contains(&self, Id(id): Id) -> bool {
        self.ids.contains(id)
    }

    #[cfg(test)]
    fn assert_invariant(&self) {
        // All elements less than start should be filled.
        for id in 0..self.start {
            assert!(self.ids.contains(id));
        }
        // All elements should be less than end.
        let end = self.values.len();
        for id in &self.ids {
            assert!(id < end)
        }
        // End should be minimal.
        assert!(end == 0 || self.ids.contains(end - 1));
    }
}

impl<T> Drop for IdMap<T> {
    fn drop(&mut self) {
        // Our vec contains uninitialized values so we must manually drop it.
        unsafe {
            for id in &self.ids {
                ptr::drop_in_place(self.values.get_unchecked_mut(id))
            }
            self.values.set_len(0);
        }
    }
}

impl<T: Clone> Clone for IdMap<T> {
    fn clone(&self) -> Self {
        self.values().cloned().collect()
    }
}

impl<T: fmt::Debug> fmt::Debug for IdMap<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{")?;
        let mut iter = self.iter();
        if let Some((Id(id), val)) = iter.next() {
            write!(f, "{:?}: {:?}", id, val)?;
            for (Id(id), val) in iter {
                write!(f, ", {:?}: {:?}", id, val)?;
            }
        }
        write!(f, "}}")
    }
}

impl<T: Eq> Eq for IdMap<T> {}

impl<T: PartialEq> PartialEq for IdMap<T> {
    fn eq(&self, other: &Self) -> bool {
        let (mut lhs, mut rhs) = (self.values(), other.values());
        loop {
            match (lhs.next(), rhs.next()) {
                (Some(left), Some(right)) if left == right => continue,
                (None, None) => return true,
                _ => return false,
            }
        }
    }
}

impl<T> Extend<T> for IdMap<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for val in iter {
            self.insert(val);
        }
    }
}

impl<T> FromIterator<T> for IdMap<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let values = Vec::from_iter(iter);
        let start = values.len();
        let ids = BitSet::from_bit_vec(BitVec::from_elem(start, true));
        IdMap { values, start, ids }
    }
}

impl<'a, T> IntoIterator for &'a IdMap<T> {
    type Item = (Id, &'a T);
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T> Index<Id> for IdMap<T> {
    type Output = T;

    fn index(&self, Id(id): Id) -> &Self::Output {
        assert!(self.ids.contains(id), "id {} out of bounds", id);
        unsafe { self.values.get_unchecked(id) }
    }
}

impl<T> IndexMut<Id> for IdMap<T> {
    fn index_mut(&mut self, Id(id): Id) -> &mut Self::Output {
        assert!(self.ids.contains(id), "id {} out of bounds", id);
        unsafe { self.values.get_unchecked_mut(id) }
    }
}

#[derive(Clone)]
/// Iterator over ids.
pub struct Ids<'a> {
    ids: bit_set::Iter<'a, u32>,
    start: usize,
}

impl<'a> Iterator for Ids<'a> {
    type Item = Id;

    fn next(&mut self) -> Option<Self::Item> {
        self.ids.next().map(Id)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.start, self.ids.size_hint().1)
    }
}

/// Iterator over values.
pub struct Values<'a, T: 'a> {
    ids: bit_set::Iter<'a, u32>,
    values: &'a [T],
    start: usize,
}

impl<'a, T: 'a> Iterator for Values<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.ids.next().map(|id| unsafe { self.values.get_unchecked(id) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.start, self.ids.size_hint().1)
    }
}

impl<'a, T: 'a> Clone for Values<'a, T> {
    fn clone(&self) -> Self {
        Values {
            ids: self.ids.clone(),
            values: self.values,
            start: self.start,
        }
    }
}

/// Iterator over id, value pairs.
pub struct Iter<'a, T: 'a> {
    ids: bit_set::Iter<'a, u32>,
    values: &'a [T],
    start: usize,
}

impl<'a, T: 'a> Iterator for Iter<'a, T> {
    type Item = (Id, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.ids.next().map(|id| (Id(id), unsafe { self.values.get_unchecked(id) }))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.start, self.ids.size_hint().1)
    }
}

impl<'a, T: 'a> Clone for Iter<'a, T> {
    fn clone(&self) -> Self {
        Iter {
            ids: self.ids.clone(),
            values: self.values,
            start: self.start,
        }
    }
}