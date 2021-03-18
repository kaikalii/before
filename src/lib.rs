use std::{
    borrow::Borrow,
    cmp::Ordering,
    convert::TryInto,
    fmt,
    hash::{Hash, Hasher},
    iter::{once, FromIterator},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    str::FromStr,
};

use serde::*;

pub trait Conversion {
    type Input;
    type Output;
    fn convert(val: Self::Input) -> Self::Output;
    fn de<'de, D>(deserializer: D) -> Result<Self::Output, D::Error>
    where
        D: Deserializer<'de>,
        Self::Input: Deserialize<'de>,
        Self::Output: Deserialize<'de>,
    {
        OldOrNew::<Self::Input, Self::Output>::deserialize(deserializer)
            .map(|oon| oon.into_new(Self::convert))
    }
}

pub struct Convert<T, U>(PhantomData<(T, U)>);

impl<T, U> Conversion for Convert<T, U>
where
    T: Into<U>,
{
    type Input = T;
    type Output = U;
    fn convert(val: T) -> Self::Output {
        val.into()
    }
}

pub struct Collect<I, C>(PhantomData<(I, C)>);

impl<C, I> Conversion for Collect<I, C>
where
    C: FromIterator<I>,
{
    type Input = I;
    type Output = C;
    fn convert(val: I) -> Self::Output {
        once(val).collect()
    }
}

pub struct CollectDefaultKey<K, V, C>(PhantomData<(K, V, C)>);

impl<C, K, V> Conversion for CollectDefaultKey<K, V, C>
where
    K: Default,
    C: FromIterator<(K, V)>,
{
    type Input = V;
    type Output = C;
    fn convert(val: V) -> Self::Output {
        once((K::default(), val)).collect()
    }
}

pub struct TryConvert<T, U>(PhantomData<(T, U)>);

impl<T, U> Conversion for TryConvert<T, U>
where
    T: TryInto<U>,
{
    type Input = T;
    type Output = Result<U, T::Error>;
    fn convert(val: T) -> Self::Output {
        val.try_into()
    }
}

pub struct TryConvertOrDefault<T, U>(PhantomData<(T, U)>);

impl<T, U> Conversion for TryConvertOrDefault<T, U>
where
    T: TryInto<U>,
    U: Default,
{
    type Input = T;
    type Output = U;
    fn convert(val: T) -> Self::Output {
        val.try_into().unwrap_or_default()
    }
}

pub struct Parse<T>(PhantomData<T>);

impl<T> Conversion for Parse<T>
where
    T: FromStr,
{
    type Input = String;
    type Output = Result<T, T::Err>;
    fn convert(val: String) -> Self::Output {
        val.parse()
    }
}

pub struct ParseOrDefault<T>(PhantomData<T>);

impl<T> Conversion for ParseOrDefault<T>
where
    T: FromStr + Default,
{
    type Input = String;
    type Output = T;
    fn convert(val: String) -> Self::Output {
        val.parse().unwrap_or_default()
    }
}

pub struct Compose<A, B>(PhantomData<(A, B)>);

impl<A, B, T, U, V> Conversion for Compose<A, B>
where
    A: Conversion<Input = T, Output = U>,
    B: Conversion<Input = U, Output = V>,
{
    type Input = T;
    type Output = V;
    fn convert(val: T) -> Self::Output {
        B::convert(A::convert(val))
    }
}

pub struct Identity<T>(PhantomData<T>);

impl<T> Conversion for Identity<T> {
    type Input = T;
    type Output = T;
    fn convert(val: T) -> Self::Output {
        val
    }
}

pub struct Map<F, I, C>(PhantomData<(F, I, C)>);

impl<F, I, C> Conversion for Map<F, I, C>
where
    F: Conversion<Input = I::Item>,
    I: IntoIterator,
    C: FromIterator<F::Output>,
{
    type Input = I;
    type Output = C;
    fn convert(val: I) -> Self::Output {
        val.into_iter().map(F::convert).collect()
    }
}

pub struct ToString<T>(PhantomData<T>);

impl<T> Conversion for ToString<T>
where
    T: std::string::ToString,
{
    type Input = T;
    type Output = String;
    fn convert(val: T) -> Self::Output {
        val.to_string()
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OldOrNew<O, N> {
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

pub struct Legacy<C, T>
where
    T: ?Sized,
{
    pd: PhantomData<C>,
    new: T,
}

impl<C, T> Legacy<C, T> {
    pub fn into_inner(legacy: Self) -> T {
        legacy.new
    }
}

impl<C, T> fmt::Debug for Legacy<C, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.new.fmt(f)
    }
}

impl<C, T> fmt::Display for Legacy<C, T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.new.fmt(f)
    }
}

impl<C, T> Clone for Legacy<C, T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Legacy {
            pd: PhantomData,
            new: self.new.clone(),
        }
    }
}

impl<C, T> Copy for Legacy<C, T> where T: Copy {}

impl<C, T> PartialEq for Legacy<C, T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.new == other.new
    }
}

impl<C, T> Eq for Legacy<C, T> where T: Eq {}

impl<C, T> PartialOrd for Legacy<C, T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.new.partial_cmp(&other.new)
    }
}

impl<C, T> Ord for Legacy<C, T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.new.cmp(&other.new)
    }
}

impl<C, T> Hash for Legacy<C, T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.new.hash(state);
    }
}

impl<C, T> Default for Legacy<C, T>
where
    T: Default,
{
    fn default() -> Self {
        Legacy {
            pd: PhantomData,
            new: T::default(),
        }
    }
}

impl<C, T> From<T> for Legacy<C, T> {
    fn from(new: T) -> Self {
        Legacy {
            pd: PhantomData,
            new,
        }
    }
}

impl<C, T> AsRef<T> for Legacy<C, T> {
    fn as_ref(&self) -> &T {
        &self.new
    }
}

impl<C, T> AsMut<T> for Legacy<C, T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.new
    }
}

impl<C, T> Borrow<T> for Legacy<C, T> {
    fn borrow(&self) -> &T {
        &self.new
    }
}

impl<C, T> Deref for Legacy<C, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.new
    }
}

impl<C, T> DerefMut for Legacy<C, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.new
    }
}

impl<C, T> Serialize for Legacy<C, T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.new.serialize(serializer)
    }
}

impl<'de, C, T> Deserialize<'de> for Legacy<C, T>
where
    T: Deserialize<'de>,
    C: Conversion<Output = T>,
    C::Input: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        OldOrNew::<C::Input, T>::deserialize(deserializer).map(|oon| Legacy {
            new: oon.into_new(C::convert),
            pd: PhantomData,
        })
    }
}

#[test]
fn simple_test() {
    #[derive(Serialize)]
    struct OldFoo {
        val: u64,
    }
    #[derive(Debug, PartialEq, Deserialize)]
    struct NewFoo {
        #[serde(
            deserialize_with = "Compose::<TryConvertOrDefault::<u64, _>, Collect::<_, _>>::de",
            alias = "val"
        )]
        vals: Vec<u32>,
    }

    let old_foo_string = serde_json::to_string(&OldFoo { val: 5 }).unwrap();
    let new_foo: NewFoo = serde_json::from_str(&old_foo_string).unwrap();
    assert_eq!(new_foo, NewFoo { vals: vec![5] })
}

#[test]
fn legacy_test() {
    let x = serde_json::from_str::<Legacy<ParseOrDefault<_>, u32>>(r#""5""#).unwrap();
    assert_eq!(*x, 5);
}
