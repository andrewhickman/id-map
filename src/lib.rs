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

    /// Remove all values from the map.
    pub fn clear(&mut self) {
        unsafe { self.drop_values() }
        self.ids.clear();
    }

    /// Inserts a value into the map and returns its id.
    pub fn insert(&mut self, val: T) -> Id {
        if self.space == self.values.len() {
            self.values.push(val)
        } else {
            unsafe { ptr::write(self.values.get_unchecked_mut(self.space), val) }
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

    /// An iterator over ids.
    pub fn ids(&self) -> Ids {
        Ids {
            ids: self.ids.iter(),
            min: self.space,
        }
    }

    /// An iterator over values.
    pub fn values(&self) -> Values<T> {
        Values {
            ids: self.ids.iter(),
            values: &self.values,
            min: self.space,
        }
    }

    /// A mutable iterator over values.
    pub fn values_mut(&mut self) -> ValuesMut<T> {
        ValuesMut {
            ids: self.ids.iter(),
            values: &mut self.values,
            min: self.space,
        }
    }

    /// An iterator over id-value pairs.
    pub fn iter(&self) -> Iter<T> {
        Iter {
            ids: self.ids.iter(),
            values: &self.values,
            min: self.space,
        }
    }

    /// A mutable iterator over id-value pairs.
    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            ids: self.ids.iter(),
            values: &mut self.values,
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
        // values.len() should be the least upper bound on ids.
        for id in &self.ids {
            assert!(id < self.values.len())
        }
        assert!(self.values.len() == 0 || self.ids.contains(self.values.len() - 1));
    }

    /// Clear the values vec. Unsafe since ids is not updated.
    unsafe fn drop_values(&mut self) {
        for id in &self.ids {
            ptr::drop_in_place(self.values.get_unchecked_mut(id))
        }
        self.values.set_len(0);
    }
}

impl<T> Drop for IdMap<T> {
    fn drop(&mut self) {
        unsafe { self.drop_values() }
    }
}

impl<T: Clone> Clone for IdMap<T> {
    fn clone(&self) -> Self {
        let ids = self.ids.clone();
        let len = self.values.len();
        let mut values = Vec::with_capacity(len);
        unsafe {
            values.set_len(len);
            for id in &ids {
                ptr::write(values.get_unchecked_mut(id),
                           self.values.get_unchecked(id).clone());
            }
        }
        IdMap {
            ids,
            values,
            space: len,
        }
    }

    fn clone_from(&mut self, other: &Self) {
        self.clear();
        let len = other.values.len();
        self.ids.clone_from(&other.ids);
        self.values.reserve(len);
        unsafe {
            self.values.set_len(len);
            for id in &self.ids {
                ptr::write(self.values.get_unchecked_mut(id),
                           other.values.get_unchecked(id).clone());
            }
        }
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
        self.ids == other.ids &&
        self.ids
            .iter()
            .zip(other.ids.iter())
            .all(|(l, r)| unsafe { self.values.get_unchecked(l) == other.values.get_unchecked(r) })
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

impl<'a, T> IntoIterator for &'a mut IdMap<T> {
    type Item = (Id, &'a mut T);
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
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
/// An iterator over all ids.
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

/// A mutable iterator over all values.
pub struct ValuesMut<'a, T: 'a> {
    ids: bit_set::Iter<'a, u32>,
    values: &'a mut [T],
    min: usize,
}

impl<'a, T: 'a> Iterator for ValuesMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        // Cast through a pointer to get rid of lifetime information. This is effectively asserting
        // that each reference returned is distinct.
        self.ids.next().map(|id| unsafe { &mut *(self.values.get_unchecked_mut(id) as *mut T) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.min, self.ids.size_hint().1)
    }
}

/// An iterator over id-value pairs.
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

/// A mutable iterator over id-value pairs.
pub struct IterMut<'a, T: 'a> {
    ids: bit_set::Iter<'a, u32>,
    values: &'a mut [T],
    min: usize,
}

impl<'a, T: 'a> Iterator for IterMut<'a, T> {
    type Item = (Id, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        self.ids.next().map(|id| {
                                (Id(id),
                                 unsafe { &mut *(self.values.get_unchecked_mut(id) as *mut T) })
                            })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.min, self.ids.size_hint().1)
    }
}
