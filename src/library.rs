use std::{collections::HashMap, hash::Hash, iter::FusedIterator};

use brownstone::move_builder::{ArrayBuilder, PushResult};

#[macro_export]
macro_rules! express {
    ($receiver:ident $(.$method:ident($($args:tt)*))*) => {
        {
            let mut receiver = $receiver;
            $(
                receiver.$method($($args)*);
            )*
            receiver
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counter<T: Hash + Eq> {
    counts: HashMap<T, usize>,
}

impl<T: Hash + Eq> Counter<T> {
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.counts.len()
    }

    pub fn contains(&self, value: &T) -> bool {
        self.counts.contains_key(value)
    }

    pub fn items(&self) -> impl Iterator<Item = &T> + FusedIterator + ExactSizeIterator + Clone {
        self.counts.keys()
    }

    pub fn add(&mut self, item: T, count: usize) {
        *self.counts.entry(item).or_insert(0) += count
    }
}

impl<T: Eq + Hash> Default for Counter<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Hash + Eq> Extend<(T, usize)> for Counter<T> {
    fn extend<I: IntoIterator<Item = (T, usize)>>(&mut self, iter: I) {
        iter.into_iter()
            .for_each(|(item, count)| self.add(item, count))
    }
}

impl<T: Hash + Eq> Extend<T> for Counter<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.extend(iter.into_iter().map(|item| (item, 1)))
    }
}

impl<T: Hash + Eq, U> FromIterator<U> for Counter<T>
where
    Self: Extend<U>,
{
    fn from_iter<I: IntoIterator<Item = U>>(iter: I) -> Self {
        let mut this = Self::new();
        this.extend(iter);
        this
    }
}

#[derive(Debug, Default, Clone)]
pub struct Chunks<I, const N: usize> {
    iterator: I,
}

impl<I: Iterator, const N: usize> Iterator for Chunks<I, N> {
    type Item = [I::Item; N];

    fn next(&mut self) -> Option<Self::Item> {
        Some(brownstone::build![self.iterator.next()?])
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (min, max) = self.iterator.size_hint();

        (min / N, max.map(|max| max / N))
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.iterator.count() / N
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let builder = match ArrayBuilder::start() {
            PushResult::Full(array) => return Some(array),
            PushResult::NotFull(builder) => builder,
        };

        let n = n.checked_mul(N).expect("usize overflow");

        let mut builder = match builder.push(self.iterator.nth(n)?) {
            PushResult::Full(array) => return Some(array),
            PushResult::NotFull(builder) => builder,
        };

        loop {
            builder = match builder.push(self.iterator.next()?) {
                PushResult::Full(array) => return Some(array),
                PushResult::NotFull(builder) => builder,
            }
        }
    }

    fn fold<B, F>(self, init: B, mut func: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        let builder = match ArrayBuilder::start() {
            PushResult::Full(_array) => panic!("called Chunks::fold but N is 0"),
            PushResult::NotFull(builder) => builder,
        };

        let (_, accum) =
            self.iterator
                .fold((builder, init), |(builder, accum), item| {
                    match builder.push(item) {
                        PushResult::NotFull(builder) => (builder, accum),
                        PushResult::Full(array) => match ArrayBuilder::start() {
                            PushResult::Full(_arr) => unreachable!(),
                            PushResult::NotFull(builder) => (builder, func(accum, array)),
                        },
                    }
                });

        accum
    }
}

impl<T: FusedIterator, const N: usize> FusedIterator for Chunks<T, N> {}

impl<T: ExactSizeIterator, const N: usize> ExactSizeIterator for Chunks<T, N> {
    fn len(&self) -> usize {
        self.iterator.len() / N
    }
}

pub trait IterExt: Iterator + Sized {
    fn streaming_chunks<const N: usize>(self) -> Chunks<Self, N> {
        Chunks { iterator: self }
    }
}

impl<T: Iterator + Sized> IterExt for T {}

#[macro_export]
macro_rules! parser {
    (
        $(
            $parser:expr $(=> $bind:ident)?
        ),* ;
        $map:expr
    ) => {
        move |input| -> nom::IResult<_, _, _> {
            $(
                let mut parser = $parser;
                let (input, value) = parser.parse(input)?;
                $(
                    let $bind = value;
                    let value = ();
                )?
                drop(value);
            )*

            Ok((input, $map))
        }
    };
}
