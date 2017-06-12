use super::*;

#[test]
fn id_map() {
    let mut ids = IdMap::<Box<u32>>::new();
    ids.assert_invariant();

    assert_eq!(ids.insert(Box::new(0)), Id(0));
    assert_eq!(*ids[Id(0)], 0);
    ids.assert_invariant();

    assert_eq!(ids.insert(Box::new(1)), Id(1));
    assert_eq!(*ids[Id(1)], 1);
    ids.assert_invariant();

    assert_eq!(ids.insert(Box::new(2)), Id(2));
    assert_eq!(*ids[Id(2)], 2);
    ids.assert_invariant();

    assert_eq!(*ids.remove(Id(0)).unwrap(), 0);
    assert_eq!(*ids.remove(Id(2)).unwrap(), 2);
    ids.assert_invariant();

    assert_eq!(ids.insert(Box::new(3)), Id(0));
    assert_eq!(*ids[Id(0)], 3);
    ids.assert_invariant();

    assert_eq!(ids.insert(Box::new(4)), Id(2));
    assert_eq!(*ids[Id(2)], 4);
    ids.assert_invariant();
}

#[test]
fn iter() {
    let ids1 = IdMap::from_iter(0..5);
    ids1.assert_invariant();

    assert_eq!(ids1[Id(0)], 0);
    assert_eq!(ids1[Id(1)], 1);
    assert_eq!(ids1[Id(2)], 2);
    assert_eq!(ids1[Id(3)], 3);
    assert_eq!(ids1[Id(4)], 4);

    let ids2: IdMap<_> = ids1.clone();
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

    ids[Id(0)] = 6;
}
