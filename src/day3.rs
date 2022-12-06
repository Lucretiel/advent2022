use std::str::FromStr;

use anyhow::Context;
use itertools::{process_results, Itertools};
use lazy_format::lazy_format;

use crate::library::{Counter, IterExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Item {
    id: u8,
}

impl Item {
    pub fn new(id: u8) -> Option<Self> {
        id.is_ascii_alphabetic().then_some(Item { id })
    }

    pub fn value(self) -> i64 {
        if self.id.is_ascii_lowercase() {
            (self.id - b'a' + 1) as i64
        } else if self.id.is_ascii_uppercase() {
            (self.id - b'A' + 27) as i64
        } else {
            unreachable!()
        }
    }
}

#[derive(Debug, Clone, Default)]
struct Compartment {
    items: Counter<Item>,
}

impl FromStr for Compartment {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        s.as_bytes()
            .iter()
            .copied()
            .enumerate()
            .map(|(idx, id)| {
                Item::new(id).with_context(|| {
                    lazy_format!("item '{}' at index {idx} is not a valid Item", id as char)
                })
            })
            .try_collect()
            .map(|items| Compartment { items })
    }
}

struct Sack {
    first: Compartment,
    second: Compartment,
}

impl Sack {
    pub fn shared(&self) -> impl Iterator<Item = Item> + '_ {
        let (iter, filter) = match self.first.items.len() >= self.second.items.len() {
            true => (&self.first, &self.second),
            false => (&self.second, &self.first),
        };

        iter.items
            .items()
            .copied()
            .filter(|item| filter.items.contains(item))
    }

    pub fn contains(&self, item: Item) -> bool {
        self.first.items.contains(&item) || self.second.items.contains(&item)
    }

    pub fn items(&self) -> impl Iterator<Item = Item> + '_ {
        self.first
            .items
            .items()
            .chain(self.second.items.items())
            .copied()
    }
}

impl FromStr for Sack {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let mid = s.len() / 2;
        let (s1, s2) = s.split_at(mid);

        Ok(Self {
            first: s1.parse().context("failed to parse first compartment")?,
            second: s2.parse().context("failed to parse second compartment")?,
        })
    }
}

pub fn part1(input: &str) -> anyhow::Result<i64> {
    let sacks = input.lines().enumerate().map(|(index, line)| {
        line.parse()
            .context(lazy_format!("failed to parse sack on line {}", index + 1))
    });

    process_results(sacks, |sacks| {
        sacks
            .map(|sack: Sack| sack.shared().map(|item| item.value()).sum::<i64>())
            .sum()
    })
}
pub fn part2(input: &str) -> anyhow::Result<i64> {
    let sacks = input.lines().enumerate().map(|(index, line)| {
        line.parse()
            .context(lazy_format!("failed to parse sack on line {}", index + 1))
    });

    process_results(sacks, |sacks| {
        sacks
            .streaming_chunks()
            .map(|[a, b, c]: [Sack; 3]| {
                let items = a.items();
                items
                    .filter(|&item| b.contains(item))
                    .filter(|&item| c.contains(item))
                    .map(|common_item| common_item.value())
                    .sum::<i64>()
            })
            .sum()
    })
}
