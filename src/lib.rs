use std::{
    convert::TryInto,
    iter::{once, FromIterator},
    marker::PhantomData,
    str::FromStr,
};

use serde::*;

pub trait ItemBuilder<V> {
    type Item;
    fn build_item(val: V) -> Self::Item;
    fn build<'de, D>(deserializer: D) -> Result<Self::Item, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de>,
        Self::Item: Deserialize<'de>,
    {
        Ok(
            match OldOrNew::<V, Self::Item>::deserialize(deserializer)? {
                OldOrNew::Old(old) => Self::build_item(old),
                OldOrNew::New(new) => new,
            },
        )
    }
    fn convert<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de>,
        T: From<Self::Item> + Deserialize<'de>,
    {
        Ok(match OldOrNew::<V, T>::deserialize(deserializer)? {
            OldOrNew::Old(old) => Self::build_item(old).into(),
            OldOrNew::New(new) => new,
        })
    }
    fn collect<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de>,
        T: FromIterator<Self::Item> + Deserialize<'de>,
    {
        Ok(match OldOrNew::<V, T>::deserialize(deserializer)? {
            OldOrNew::Old(val) => once(Self::build_item(val)).collect(),
            OldOrNew::New(collection) => collection,
        })
    }
}

pub struct SingleValue(PhantomData<()>);

impl<V> ItemBuilder<V> for SingleValue {
    type Item = V;
    fn build_item(val: V) -> Self::Item {
        val
    }
}

pub struct KeyValue<K>(PhantomData<K>);

impl<K, V> ItemBuilder<V> for KeyValue<K>
where
    K: Default,
{
    type Item = (K, V);
    fn build_item(val: V) -> Self::Item {
        (K::default(), val)
    }
}

pub struct ParseOrDefault<T>(PhantomData<T>);

impl<T> ItemBuilder<String> for ParseOrDefault<T>
where
    T: FromStr + Default,
{
    type Item = T;
    fn build_item(val: String) -> Self::Item {
        val.parse().unwrap_or_default()
    }
}

pub struct TryIntoOrDefault<T, U>(PhantomData<(T, U)>);

impl<T, U> ItemBuilder<T> for TryIntoOrDefault<T, U>
where
    T: TryInto<U>,
    U: Default,
{
    type Item = U;
    fn build_item(val: T) -> Self::Item {
        val.try_into().unwrap_or_default()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum OldOrNew<O, N> {
    Old(O),
    New(N),
}

#[test]
fn test() {
    #[derive(Serialize)]
    struct OldFoo {
        val: u64,
    }
    #[derive(Debug, PartialEq, Deserialize)]
    struct NewFoo {
        #[serde(
            deserialize_with = "TryIntoOrDefault::<u64, _>::collect",
            alias = "val"
        )]
        vals: Vec<u32>,
    }

    let old_foo_string = serde_json::to_string(&OldFoo { val: 5 }).unwrap();
    let new_foo: NewFoo = serde_json::from_str(&old_foo_string).unwrap();
    assert_eq!(new_foo, NewFoo { vals: vec![5] })
}
