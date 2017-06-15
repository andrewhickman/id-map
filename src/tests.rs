use super::*;

#[test]
fn id_map() {
    let mut ids = IdMap::<Box<u32>>::new();
    ids.assert_invariant();

    assert_eq!(ids.insert(Box::new(0)), 0);
    assert_eq!(*ids[0], 0);
    ids.assert_invariant();

    assert_eq!(ids.insert(Box::new(1)), 1);
    assert_eq!(*ids[1], 1);
    ids.assert_invariant();

    assert_eq!(ids.insert(Box::new(2)), 2);
    assert_eq!(*ids[2], 2);
    ids.assert_invariant();

    assert_eq!(*ids.remove(0).unwrap(), 0);
    assert_eq!(*ids.remove(2).unwrap(), 2);
    ids.assert_invariant();

    assert_eq!(ids.insert(Box::new(3)), 0);
    assert_eq!(*ids[0], 3);
    ids.assert_invariant();

    assert_eq!(ids.insert(Box::new(4)), 2);
    assert_eq!(*ids[2], 4);
    ids.assert_invariant();
}

#[test]
fn iter() {
    let ids1 = IdMap::from_iter(0..5);
    ids1.assert_invariant();

    assert_eq!(ids1[0], 0);
    assert_eq!(ids1[1], 1);
    assert_eq!(ids1[2], 2);
    assert_eq!(ids1[3], 3);
    assert_eq!(ids1[4], 4);

    let mut ids2: IdMap<_> = ids1.clone();
    assert_eq!(ids1, ids2);
    ids2.clear();
    ids2.clone_from(&ids1);
    assert_eq!(ids1, ids2);
}

#[test]
fn iter_mut() {
    let mut ids = IdMap::from_iter(0..5);

    let mut refs: Vec<&mut u32> = ids.values_mut().collect();

    refs.sort();
    refs.dedup_by(|l, r| *l as *mut u32 == *r as *mut u32);

    assert_eq!(refs.len(), 5)
}

#[test]
#[should_panic(expected = "id 0 out of bounds")]
fn panic() {
    let mut ids = IdMap::<u32>::new();

    ids[0] = 6;
}

#[test]
fn ubsan() {
    use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};

    static COUNT: AtomicUsize = ATOMIC_USIZE_INIT;

    struct Test(u32);

    impl Test {
        fn new(val: u32) -> Self {
            COUNT.fetch_add(1, Ordering::SeqCst);
            Test(val)
        }
    }

    impl Drop for Test {
        fn drop(&mut self) {
            assert!(COUNT.fetch_sub(1, Ordering::SeqCst) != 0);
        }
    }

    impl Clone for Test {
        fn clone(&self) -> Self {
            Self::new(self.0)
        }
    }

    let mut ids = IdMap::<Test>::new();
    ids.assert_invariant();

    assert_eq!(ids.insert(Test::new(0)), 0);
    ids.assert_invariant();

    assert_eq!(ids.insert(Test::new(1)), 1);
    ids.assert_invariant();

    assert_eq!(ids.insert(Test::new(2)), 2);
    ids.assert_invariant();

    ids.remove(0).unwrap();
    ids.assert_invariant();

    ids.remove(2).unwrap();
    ids.assert_invariant();

    assert_eq!(ids.insert(Test::new(3)), 0);
    ids.assert_invariant();

    assert_eq!(ids.insert(Test::new(4)), 2);
    ids.assert_invariant();

    std::mem::drop(ids.clone());

    let mut ids2 = IdMap::new();
    ids2.clone_from(&ids);

    ids2.clear();

    std::mem::drop(ids);

    assert_eq!(COUNT.load(Ordering::SeqCst), 0);
}

#[test]
fn print() {
    let ids = IdMap::from_iter(0..5);

    assert_eq!(format!("{:?}", ids), "{0: 0, 1: 1, 2: 2, 3: 3, 4: 4}")
}

#[test]
fn insert_at() {
    let mut ids = IdMap::from_iter(0..5);

    assert_eq!(ids.insert_at(3, 6), Some(3));
    ids.assert_invariant();
    assert_eq!(ids[3], 6);
    ids.assert_invariant();
    assert_eq!(ids.remove(3), Some(6));
    ids.assert_invariant();
    assert_eq!(ids.insert_at(3, 7), None);
    ids.assert_invariant();
    assert_eq!(ids.insert_at(10, 10), None);
    ids.assert_invariant();
    assert_eq!(ids.remove(10), Some(10));
    ids.assert_invariant();
}
