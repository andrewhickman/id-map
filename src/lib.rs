//! [`IdMap`] is a container that gives each item a unique id. Adding and removing by index is O(1).
//!
//! # Examples
//!
//! ```
//! # use id_map::IdMap;
//! #
//! let mut map = IdMap::new();
//! let blue_id = map.insert("blue");
//! let red_id = map.insert("red");
//!
//! map.retain(|_, &color| color != "red");
//!
//! assert!(!map.contains(red_id));
//! assert_eq!(map[blue_id], "blue");
//! ```
//!
//! [`IdMap`]: struct.IdMap.html

#![deny(missing_docs, missing_debug_implementations, unsafe_code)]

extern crate id_set;

#[cfg(test)]
mod tests;

pub use id_set::Id;

use std::iter::FromIterator;
use std::ops::{Index, IndexMut};
use std::{cmp, fmt, mem};
use std::{slice, vec};

use id_set::IdSet;

/// A container that gives each item a unique id. Internally all elements are stored contiguously.
#[derive(Clone)]
pub struct IdMap<T> {
    // The set of valid indices for values.
    ids: IdSet,
    // The buffer of values. Indices not in ids are invalid.
    values: Vec<Option<T>>,
    // The smallest empty space in the vector of values, or values.len() if no space is left.
    space: Id,
}

impl<T> IdMap<T> {
    #[inline]
    /// Creates an empty `IdMap<T>`.
    pub fn new() -> Self {
        IdMap {
            ids: IdSet::new(),
            values: Vec::new(),
            space: 0,
        }
    }

    #[inline]
    /// Creates an `IdMap<T>` with the specified capacity.
    pub fn with_capacity(cap: usize) -> Self {
        IdMap {
            ids: IdSet::with_capacity(cap),
            values: Vec::with_capacity(cap),
            space: 0,
        }
    }

    #[inline]
    /// Removes all values from the map.
    pub fn clear(&mut self) {
        self.drop_values();
        self.ids.clear();
    }

    #[inline]
    /// Returns the id that a subsequent call to insert() will produce.
    pub fn next_id(&self) -> Id {
        self.space
    }

    #[inline]
    /// Returns the number of id-value pairs in the map.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    #[inline]
    /// Returns the number of id-value pairs the map can hold before reallocating.
    pub fn capacity(&self) -> usize {
        self.ids.capacity()
    }

    #[inline]
    /// Resizes the map such that that `capacity() >= cap`.
    pub fn reserve(&mut self, cap: usize) {
        self.ids.reserve(cap);
        self.values.reserve(cap);
    }

    #[inline]
    /// Resizes the map to minimize allocated memory.
    pub fn shrink_to_fit(&mut self) {
        self.ids.shrink_to_fit();
        self.values.truncate(self.ids.capacity());
        self.values.shrink_to(self.ids.capacity());
    }

    #[inline]
    /// Returns a reference to the set of valid ids.
    pub fn as_set(&self) -> &IdSet {
        &self.ids
    }

    #[inline]
    /// Inserts a value into an empty slot in the map and returns its id.
    pub fn insert(&mut self, val: T) -> Id {
        let id = self.space;
        if id == self.values.len() {
            self.values.resize_with(id + 1, Default::default);
        }
        self.values[id] = Some(val);
        self.ids.insert(id);
        self.find_space();
        id
    }

    #[inline]
    /// Inserts a value at a specific id, returning the old value if it existed.
    pub fn insert_at(&mut self, id: Id, val: T) -> Option<T> {
        if self.ids.insert(id) {
            // val was not previously in the map.
            if id == self.space {
                self.find_space();
            }
            if self.values.len() < id + 1 {
                self.values.resize_with(id + 1, Default::default);
            }
            self.values[id] = Some(val);
            None
        } else {
            // val was previously in the map
            Some(mem::replace(&mut self.values[id].as_mut().unwrap(), val))
        }
    }

    #[inline]
    /// Removes an id from the map, returning its value if it was previously in the map.
    pub fn remove(&mut self, id: Id) -> Option<T> {
        if self.ids.remove(id) {
            self.space = cmp::min(self.space, id);
            self.values[id].take()
        } else {
            None
        }
    }

    #[inline]
    /// If the id has a value, returns it, otherwise inserts a new value.
    pub fn get_or_insert(&mut self, id: Id, val: T) -> &mut T {
        self.get_or_insert_with(id, || val)
    }

    #[inline]
    /// If the id has a value, returns it, otherwise inserts a new value with the provided closure.
    pub fn get_or_insert_with<F: FnOnce() -> T>(&mut self, id: Id, f: F) -> &mut T {
        if self.ids.insert(id) {
            // val was not previously in the map.
            if id == self.space {
                self.find_space();
            }
            if self.values.len() < id + 1 {
                self.values.resize_with(id + 1, Default::default);
            }
            self.values[id] = Some(f());
        }

        self.values[id].as_mut().unwrap()
    }

    #[inline]
    /// Removes all ids in the set from the map.
    pub fn remove_set(&mut self, set: &IdSet) {
        {
            let mut iter = self.ids.intersection(set).into_iter();

            if let Some(first) = iter.next() {
                // Set iterators are increasing so we only need to change start once.
                self.space = cmp::min(self.space, first);
                self.values[first] = None;
                for id in iter {
                    self.values[id] = None;
                }
            }
        }

        self.ids.inplace_difference(set);
    }

    #[inline]
    /// Remove all values not satisfying the predicate.
    pub fn retain<F: FnMut(Id, &T) -> bool>(&mut self, mut pred: F) {
        let ids = &mut self.ids;
        let values = &mut self.values;
        let space = &mut self.space;
        ids.retain(|id| {
            if pred(id, values[id].as_ref().unwrap()) {
                true
            } else {
                *space = cmp::min(*space, id);
                values[id] = None;
                false
            }
        })
    }

    #[inline]
    /// Returns true if the map contains a value for the specified id.
    pub fn contains(&self, id: Id) -> bool {
        self.ids.contains(id)
    }

    #[inline]
    /// Returns a reference to the value at the specified id if it is in the map.
    pub fn get(&self, id: Id) -> Option<&T> {
        if self.ids.contains(id) {
            Some(self.values[id].as_ref().unwrap())
        } else {
            None
        }
    }

    #[inline]
    /// Returns a mutable reference to the value at the specified id if it is in the map.
    pub fn get_mut(&mut self, id: Id) -> Option<&mut T> {
        if self.ids.contains(id) {
            Some(self.values[id].as_mut().unwrap())
        } else {
            None
        }
    }

    #[inline]
    /// An iterator over ids, in increasing order.
    pub fn ids(&self) -> Ids {
        Ids {
            ids: self.ids.iter(),
        }
    }

    #[inline]
    /// An iterator over values, in order of increasing id.
    pub fn values(&self) -> Values<T> {
        Values {
            ids: self.ids.iter(),
            values: &self.values,
        }
    }

    #[inline]
    /// A mutable iterator over values, in order of increasing id.
    pub fn values_mut(&mut self) -> ValuesMut<T> {
        ValuesMut {
            ids: self.ids.iter(),
            prev: None,
            values: self.values.iter_mut(),
        }
    }

    #[inline]
    /// An iterator over id-value pairs, in order of increasing id.
    pub fn iter(&self) -> Iter<T> {
        Iter {
            ids: self.ids.iter(),
            values: &self.values,
        }
    }

    #[inline]
    /// A mutable iterator over id-value pairs, in order of increasing id.
    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            ids: self.ids.iter(),
            prev: None,
            values: self.values.iter_mut(),
        }
    }

    #[inline]
    /// A consuming iterator over id-value pairs, in order of increasing id.
    pub fn into_iter(self) -> IntoIter<T> {
        IntoIter {
            ids: self.ids.into_iter(),
            prev: None,
            values: self.values.into_iter(),
        }
    }

    #[cfg(test)]
    fn assert_invariant(&self) {
        // space should be the minimal empty space.
        for id in 0..self.space {
            assert!(self.ids.contains(id));
        }
        assert!(!self.ids.contains(self.space));
        // values.len() should be an upper bound on ids.
        for id in &self.ids {
            assert!(id < self.values.len())
        }
    }

    /// Clear the values vec.
    fn drop_values(&mut self) {
        for id in &self.ids {
            self.values[id] = None;
        }
    }

    /// Find the next empty space after one has been filled.
    fn find_space(&mut self) {
        // Each id corresponds to an entry in the storage so ids can never fill up.
        self.space += 1;
        while self.ids.contains(self.space) {
            self.space += 1;
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for IdMap<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{")?;
        let mut iter = self.iter();
        if let Some((id, val)) = iter.next() {
            write!(f, "{:?}: {:?}", id, val)?;
            for (id, val) in iter {
                write!(f, ", {:?}: {:?}", id, val)?;
            }
        }
        write!(f, "}}")
    }
}

impl<T> Default for IdMap<T> {
    #[inline]
    fn default() -> Self {
        IdMap::new()
    }
}

impl<T: Eq> Eq for IdMap<T> {}

impl<T: PartialEq> PartialEq for IdMap<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ids == other.ids
            && self
                .ids
                .iter()
                .zip(&other.ids)
                .all(|(l, r)| self.values[l].as_ref().unwrap() == other.values[r].as_ref().unwrap())
    }
}

impl<T> Extend<T> for IdMap<T> {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for val in iter {
            self.insert(val);
        }
    }
}

impl<T> FromIterator<T> for IdMap<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let values = Vec::from_iter(iter.into_iter().map(Some));
        let space = values.len();
        let ids = IdSet::new_filled(values.len());
        IdMap { values, space, ids }
    }
}

impl<T> FromIterator<(Id, T)> for IdMap<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = (Id, T)>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let mut map = IdMap::with_capacity(iter.size_hint().0);
        for (id, val) in iter {
            map.insert_at(id, val);
        }
        map
    }
}

impl<'a, T> IntoIterator for &'a IdMap<T> {
    type Item = (Id, &'a T);
    type IntoIter = Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut IdMap<T> {
    type Item = (Id, &'a mut T);
    type IntoIter = IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T> IntoIterator for IdMap<T> {
    type Item = (Id, T);
    type IntoIter = IntoIter<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.into_iter()
    }
}

impl<T> Index<Id> for IdMap<T> {
    type Output = T;

    #[inline]
    fn index(&self, id: Id) -> &Self::Output {
        assert!(self.ids.contains(id), "id {} out of bounds", id);
        self.values[id].as_ref().unwrap()
    }
}

impl<T> IndexMut<Id> for IdMap<T> {
    #[inline]
    fn index_mut(&mut self, id: Id) -> &mut Self::Output {
        assert!(self.ids.contains(id), "id {} out of bounds", id);
        self.values[id].as_mut().unwrap()
    }
}

#[derive(Clone, Debug)]
/// An iterator over all ids, in increasing order.
pub struct Ids<'a> {
    ids: id_set::Iter<'a>,
}

impl<'a> Iterator for Ids<'a> {
    type Item = Id;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.ids.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ids.size_hint()
    }
}

impl<'a> ExactSizeIterator for Ids<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.ids.len()
    }
}

#[derive(Debug)]
/// An iterator over all values, in order of increasing id.
pub struct Values<'a, T: 'a> {
    ids: id_set::Iter<'a>,
    values: &'a [Option<T>],
}

impl<'a, T: 'a> Iterator for Values<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.ids.next().map(|id| self.values[id].as_ref().unwrap())
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ids.size_hint()
    }
}

impl<'a, T: 'a> ExactSizeIterator for Values<'a, T> {
    #[inline]
    fn len(&self) -> usize {
        self.ids.len()
    }
}

impl<'a, T: 'a> Clone for Values<'a, T> {
    #[inline]
    fn clone(&self) -> Self {
        Values {
            ids: self.ids.clone(),
            values: self.values,
        }
    }
}

#[derive(Debug)]
/// A mutable iterator over all values, in order of increasing id.
pub struct ValuesMut<'a, T: 'a> {
    ids: id_set::Iter<'a>,
    prev: Option<Id>,
    values: slice::IterMut<'a, Option<T>>,
}

impl<'a, T: 'a> Iterator for ValuesMut<'a, T> {
    type Item = &'a mut T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let id = self.ids.next()?;
        let n = match self.prev {
            Some(prev) => id - prev - 1,
            None => 0,
        };
        self.prev = Some(id);

        Some(self.values.nth(n).unwrap().as_mut().unwrap())
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ids.size_hint()
    }
}

impl<'a, T: 'a> ExactSizeIterator for ValuesMut<'a, T> {
    #[inline]
    fn len(&self) -> usize {
        self.ids.len()
    }
}

#[derive(Debug)]
/// An iterator over id-value pairs, in order of increasing id.
pub struct Iter<'a, T: 'a> {
    ids: id_set::Iter<'a>,
    values: &'a [Option<T>],
}

impl<'a, T: 'a> Iterator for Iter<'a, T> {
    type Item = (Id, &'a T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.ids
            .next()
            .map(|id| (id, self.values[id].as_ref().unwrap()))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ids.size_hint()
    }
}

impl<'a, T: 'a> ExactSizeIterator for Iter<'a, T> {
    #[inline]
    fn len(&self) -> usize {
        self.ids.len()
    }
}

impl<'a, T: 'a> Clone for Iter<'a, T> {
    #[inline]
    fn clone(&self) -> Self {
        Iter {
            ids: self.ids.clone(),
            values: self.values,
        }
    }
}

#[derive(Debug)]
/// A mutable iterator over id-value pairs, in order of increasing id.
pub struct IterMut<'a, T: 'a> {
    ids: id_set::Iter<'a>,
    prev: Option<Id>,
    values: slice::IterMut<'a, Option<T>>,
}

impl<'a, T: 'a> Iterator for IterMut<'a, T> {
    type Item = (Id, &'a mut T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let id = self.ids.next()?;
        let n = match self.prev {
            Some(prev) => id - prev - 1,
            None => 0,
        };
        self.prev = Some(id);

        Some((
            id,
            self.values.nth(n).unwrap().as_mut().expect("id not in map"),
        ))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ids.size_hint()
    }
}

impl<'a, T: 'a> ExactSizeIterator for IterMut<'a, T> {
    #[inline]
    fn len(&self) -> usize {
        self.ids.len()
    }
}

#[derive(Clone, Debug)]
/// A consuming iterator over id-value pairs, in order of increasing id.
pub struct IntoIter<T> {
    ids: id_set::IntoIter,
    prev: Option<Id>,
    values: vec::IntoIter<Option<T>>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = (Id, T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let id = self.ids.next()?;
        let n = match self.prev {
            Some(prev) => id - prev - 1,
            None => 0,
        };
        self.prev = Some(id);

        Some((id, self.values.nth(n).unwrap().expect("id not in map")))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ids.size_hint()
    }
}
