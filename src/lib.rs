//! A container that gives each item a unique id. Adding and removing by index is O(1).

#![deny(missing_docs)]

extern crate bit_set;
extern crate bit_vec;

#[cfg(test)]
mod tests;

use std::iter::FromIterator;
use std::ops::{Index, IndexMut};
use std::{fmt, ptr};

use bit_set::BitSet;
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
    // The smallest empty space in the vector of values, or values.len() if no space is left.
    space: usize,
}

impl<T> IdMap<T> {
    /// Creates an empty `IdMap<T>`.
    pub fn new() -> Self {
        IdMap {
            ids: BitSet::new(),
            values: Vec::new(),
            space: 0,
        }
    }

    /// Creates an `IdMap<T>` with the specified capacity.
    pub fn with_capacity(cap: usize) -> Self {
        IdMap {
            ids: BitSet::with_capacity(cap),
            values: Vec::with_capacity(cap),
            space: 0,
        }
    }

    /// Inserts a value into the map and returns its id.
    pub fn insert(&mut self, val: T) -> Id {
        if self.space == self.values.len() {
            self.values.push(val)
        } else {
            unsafe {
                ptr::write(self.values.get_unchecked_mut(self.space), val)
            }
        }
        let id = self.space;
        self.ids.insert(id);

        // Find the next empty space.
        self.space += 1;
        while self.ids.contains(self.space) {
            self.space += 1;
        }

        Id(id)
    }

    /// Removes an id from the map, returning its value if it was previously in the map.
    pub fn remove(&mut self, Id(id): Id) -> Option<T> {
        if self.ids.remove(id) {
            if id < self.space {
                self.space = id;
            }
            if id + 1 == self.values.len() {
                self.values.pop()
            } else {
                Some(unsafe { ptr::read(self.values.get_unchecked(id)) })
            }
        } else {
            None
        }
    }

    /// An iterator over all ids in increasing order.
    pub fn ids(&self) -> Ids {
        Ids {
            ids: self.ids.iter(),
            min: self.space,
        }
    }

    /// An iterator over all values.
    pub fn values(&self) -> Values<T> {
        Values {
            ids: self.ids.iter(),
            values: &self.values,
            min: self.space,
        }
    }

    /// An iterator over id-value pairs in order of increasing ids.
    pub fn iter(&self) -> Iter<T> {
        Iter {
            ids: self.ids.iter(),
            values: &self.values,
            min: self.space,
        }
    }

    /// Returns true if the map contains a value for the specified id.
    pub fn contains(&self, Id(id): Id) -> bool {
        self.ids.contains(id)
    }

    #[cfg(test)]
    fn assert_invariant(&self) {
        // space should be the minimal empty space.
        for id in 0..self.space {
            assert!(self.ids.contains(id));
        }
        assert!(!self.ids.contains(self.space));
        // values.len() should be the least upper bounds on ids.
        for id in &self.ids {
            assert!(id < self.values.len())
        }
        assert!(self.values.len() == 0 || self.ids.contains(self.values.len() - 1));
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

impl<T> Default for IdMap<T> {
    fn default() -> Self {
        IdMap::new()
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
        let space = values.len();
        let ids = BitSet::from_bit_vec(BitVec::from_elem(values.len(), true));
        IdMap {
            values,
            space,
            ids,
        }
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
/// An iterator over all ids in increasing order.
pub struct Ids<'a> {
    ids: bit_set::Iter<'a, u32>,
    min: usize,
}

impl<'a> Iterator for Ids<'a> {
    type Item = Id;

    fn next(&mut self) -> Option<Self::Item> {
        self.ids.next().map(Id)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.min, self.ids.size_hint().1)
    }
}

/// An iterator over all values.
pub struct Values<'a, T: 'a> {
    ids: bit_set::Iter<'a, u32>,
    values: &'a [T],
    min: usize,
}

impl<'a, T: 'a> Iterator for Values<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.ids.next().map(|id| unsafe { self.values.get_unchecked(id) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.min, self.ids.size_hint().1)
    }
}

impl<'a, T: 'a> Clone for Values<'a, T> {
    fn clone(&self) -> Self {
        Values {
            ids: self.ids.clone(),
            values: self.values,
            min: self.min,
        }
    }
}

/// An iterator over id-value pairs in order of increasing ids.
pub struct Iter<'a, T: 'a> {
    ids: bit_set::Iter<'a, u32>,
    values: &'a [T],
    min: usize,
}

impl<'a, T: 'a> Iterator for Iter<'a, T> {
    type Item = (Id, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.ids.next().map(|id| (Id(id), unsafe { self.values.get_unchecked(id) }))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.min, self.ids.size_hint().1)
    }
}

impl<'a, T: 'a> Clone for Iter<'a, T> {
    fn clone(&self) -> Self {
        Iter {
            ids: self.ids.clone(),
            values: self.values,
            min: self.min,
        }
    }
}
