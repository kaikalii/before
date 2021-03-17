use std::{collections::*, hash::Hash};

use serde::*;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum NowMany<O, M> {
    One(O),
    Many(M),
}

pub type NowHashMap<K, V> = NowMany<V, HashMap<K, V>>;
pub type NowBTreeMap<K, V> = NowMany<V, BTreeMap<K, V>>;

impl<K, V> From<NowHashMap<K, V>> for HashMap<K, V>
where
    K: Default + Eq + Hash,
{
    fn from(proxy: NowHashMap<K, V>) -> Self {
        match proxy {
            NowMany::One(val) => {
                let mut map = HashMap::new();
                map.insert(K::default(), val);
                map
            }
            NowMany::Many(map) => map,
        }
    }
}

impl<K, V> From<NowBTreeMap<K, V>> for BTreeMap<K, V>
where
    K: Default + Ord,
{
    fn from(proxy: NowBTreeMap<K, V>) -> Self {
        match proxy {
            NowMany::One(val) => {
                let mut map = BTreeMap::new();
                map.insert(K::default(), val);
                map
            }
            NowMany::Many(map) => map,
        }
    }
}
