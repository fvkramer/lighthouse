use super::*;
use int_to_bytes::int_to_bytes32;

#[derive(Clone)]
pub struct Inner {
    pub a: u64,
    pub b: u64,
    pub c: u64,
    pub d: u64,
}

impl CachedTreeHash for Inner {
    fn build_cache_bytes(&self) -> Vec<u8> {
        let mut leaves = vec![];

        leaves.append(&mut self.a.build_cache_bytes());
        leaves.append(&mut self.b.build_cache_bytes());
        leaves.append(&mut self.c.build_cache_bytes());
        leaves.append(&mut self.d.build_cache_bytes());

        merkleize(leaves)
    }

    fn max_num_leaves(&self) -> usize {
        let mut leaves = 0;
        leaves += self.a.max_num_leaves();
        leaves += self.b.max_num_leaves();
        leaves += self.c.max_num_leaves();
        leaves += self.d.max_num_leaves();
        leaves
    }

    fn num_bytes(&self) -> usize {
        let mut bytes = 0;
        bytes += self.a.num_bytes();
        bytes += self.b.num_bytes();
        bytes += self.c.num_bytes();
        bytes += self.d.num_bytes();
        bytes
    }

    fn cached_hash_tree_root(
        &self,
        other: &Self,
        cache: &mut TreeHashCache,
        chunk: usize,
    ) -> Option<usize> {
        let num_leaves = self.max_num_leaves();
        let num_nodes = num_nodes(num_leaves);
        let num_internal_nodes = num_nodes - num_leaves;

        // Skip past the internal nodes and update any changed leaf nodes.
        {
            let chunk = chunk + num_internal_nodes;
            let chunk = self.a.cached_hash_tree_root(&other.a, cache, chunk)?;
            let chunk = self.b.cached_hash_tree_root(&other.b, cache, chunk)?;
            let chunk = self.c.cached_hash_tree_root(&other.c, cache, chunk)?;
            let _chunk = self.d.cached_hash_tree_root(&other.d, cache, chunk)?;
        }

        // Iterate backwards through the internal nodes, rehashing any node where it's children
        // have changed.
        for chunk in (0..num_internal_nodes).into_iter().rev() {
            if cache.children_modified(chunk)? {
                cache.modify_chunk(chunk, &cache.hash_children(chunk)?)?;
            }
        }

        Some(chunk + num_nodes)
    }
}

fn join(many: Vec<Vec<u8>>) -> Vec<u8> {
    let mut all = vec![];
    for one in many {
        all.extend_from_slice(&mut one.clone())
    }
    all
}

#[test]
fn merkleize_odd() {
    let data = join(vec![
        int_to_bytes32(1),
        int_to_bytes32(2),
        int_to_bytes32(3),
        int_to_bytes32(4),
        int_to_bytes32(5),
    ]);

    merkleize(sanitise_bytes(data));
}

fn generic_test(index: usize) {
    let inner = Inner {
        a: 1,
        b: 2,
        c: 3,
        d: 4,
    };

    let cache = inner.build_cache_bytes();

    let changed_inner = match index {
        0 => Inner {
            a: 42,
            ..inner.clone()
        },
        1 => Inner {
            b: 42,
            ..inner.clone()
        },
        2 => Inner {
            c: 42,
            ..inner.clone()
        },
        3 => Inner {
            d: 42,
            ..inner.clone()
        },
        _ => panic!("bad index"),
    };

    let mut cache_struct = TreeHashCache::from_bytes(cache.clone()).unwrap();

    changed_inner
        .cached_hash_tree_root(&inner, &mut cache_struct, 0)
        .unwrap();

    // assert_eq!(*cache_struct.hash_count, 3);

    let new_cache: Vec<u8> = cache_struct.into();

    let data1 = int_to_bytes32(1);
    let data2 = int_to_bytes32(2);
    let data3 = int_to_bytes32(3);
    let data4 = int_to_bytes32(4);

    let mut data = vec![data1, data2, data3, data4];

    data[index] = int_to_bytes32(42);

    let expected = merkleize(join(data));

    assert_eq!(expected, new_cache);
}

#[test]
fn cached_hash_on_inner() {
    generic_test(0);
    generic_test(1);
    generic_test(2);
    generic_test(3);
}

#[test]
fn build_cache_matches_merkelize() {
    let data1 = int_to_bytes32(1);
    let data2 = int_to_bytes32(2);
    let data3 = int_to_bytes32(3);
    let data4 = int_to_bytes32(4);

    let data = join(vec![data1, data2, data3, data4]);
    let expected = merkleize(data);

    let inner = Inner {
        a: 1,
        b: 2,
        c: 3,
        d: 4,
    };

    let cache = inner.build_cache_bytes();

    assert_eq!(expected, cache);
}

#[test]
fn merkleize_4_leaves() {
    let data1 = hash(&int_to_bytes32(1));
    let data2 = hash(&int_to_bytes32(2));
    let data3 = hash(&int_to_bytes32(3));
    let data4 = hash(&int_to_bytes32(4));

    let data = join(vec![
        data1.clone(),
        data2.clone(),
        data3.clone(),
        data4.clone(),
    ]);

    let cache = merkleize(data);

    let hash_12 = {
        let mut joined = vec![];
        joined.append(&mut data1.clone());
        joined.append(&mut data2.clone());
        hash(&joined)
    };
    let hash_34 = {
        let mut joined = vec![];
        joined.append(&mut data3.clone());
        joined.append(&mut data4.clone());
        hash(&joined)
    };
    let hash_hash12_hash_34 = {
        let mut joined = vec![];
        joined.append(&mut hash_12.clone());
        joined.append(&mut hash_34.clone());
        hash(&joined)
    };

    for (i, chunk) in cache.chunks(HASHSIZE).enumerate().rev() {
        let expected = match i {
            0 => hash_hash12_hash_34.clone(),
            1 => hash_12.clone(),
            2 => hash_34.clone(),
            3 => data1.clone(),
            4 => data2.clone(),
            5 => data3.clone(),
            6 => data4.clone(),
            _ => vec![],
        };

        assert_eq!(chunk, &expected[..], "failed at {}", i);
    }
}