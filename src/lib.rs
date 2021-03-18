use std::{
    convert::TryInto,
    iter::{once, FromIterator},
    marker::PhantomData,
    str::FromStr,
};

use serde::*;

pub trait Conversion<T> {
    type Output;
    fn convert(val: T) -> Self::Output;
    fn de<'de, D>(deserializer: D) -> Result<Self::Output, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
        Self::Output: Deserialize<'de>,
    {
        OldOrNew::<T, Self::Output>::deserialize(deserializer)
            .map(|oon| oon.into_new(Self::convert))
    }
}

pub struct Convert<T, U>(PhantomData<(T, U)>);

impl<T, U> Conversion<T> for Convert<T, U>
where
    T: Into<U>,
{
    type Output = U;
    fn convert(val: T) -> Self::Output {
        val.into()
    }
}

pub struct Collect<C>(PhantomData<C>);

impl<C, I> Conversion<I> for Collect<C>
where
    C: FromIterator<I>,
{
    type Output = C;
    fn convert(val: I) -> Self::Output {
        once(val).collect()
    }
}

pub struct TryConvert<T, U>(PhantomData<(T, U)>);

impl<T, U> Conversion<T> for TryConvert<T, U>
where
    T: TryInto<U>,
{
    type Output = Result<U, T::Error>;
    fn convert(val: T) -> Self::Output {
        val.try_into()
    }
}

pub struct TryConvertOrDefault<T, U>(PhantomData<(T, U)>);

impl<T, U> Conversion<T> for TryConvertOrDefault<T, U>
where
    T: TryInto<U>,
    U: Default,
{
    type Output = U;
    fn convert(val: T) -> Self::Output {
        val.try_into().unwrap_or_default()
    }
}

pub struct Parse<T>(PhantomData<T>);

impl<T> Conversion<String> for Parse<T>
where
    T: FromStr,
{
    type Output = Result<T, T::Err>;
    fn convert(val: String) -> Self::Output {
        val.parse()
    }
}

pub struct ParseOrDefault<T>(PhantomData<T>);

impl<T> Conversion<String> for ParseOrDefault<T>
where
    T: FromStr + Default,
{
    type Output = T;
    fn convert(val: String) -> Self::Output {
        val.parse().unwrap_or_default()
    }
}

pub struct Compose<A, B>(PhantomData<(A, B)>);

impl<A, B, T, U, V> Conversion<T> for Compose<A, B>
where
    A: Conversion<T, Output = U>,
    B: Conversion<U, Output = V>,
{
    type Output = V;
    fn convert(val: T) -> Self::Output {
        B::convert(A::convert(val))
    }
}

pub struct Identity<T>(PhantomData<T>);

impl<T> Conversion<T> for Identity<T> {
    type Output = T;
    fn convert(val: T) -> Self::Output {
        val
    }
}

pub struct Map<F, C>(PhantomData<(F, C)>);

impl<F, I, C> Conversion<I> for Map<F, C>
where
    F: Conversion<I::Item>,
    I: IntoIterator,
    C: FromIterator<F::Output>,
{
    type Output = C;
    fn convert(val: I) -> Self::Output {
        val.into_iter().map(F::convert).collect()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum OldOrNew<O, N> {
    New(N),
    Old(O),
}

impl<O, N> OldOrNew<O, N> {
    pub fn into_new<F>(self, f: F) -> N
    where
        F: Fn(O) -> N,
    {
        match self {
            OldOrNew::Old(old) => f(old),
            OldOrNew::New(new) => new,
        }
    }
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
            deserialize_with = "Compose::<TryConvertOrDefault::<u64, _>, Collect::<_>>::de",
            alias = "val"
        )]
        vals: Vec<u32>,
    }

    let old_foo_string = serde_json::to_string(&OldFoo { val: 5 }).unwrap();
    let new_foo: NewFoo = serde_json::from_str(&old_foo_string).unwrap();
    assert_eq!(new_foo, NewFoo { vals: vec![5] })
}
