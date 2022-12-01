use std::cmp::max;

use anyhow::Context;
use nom::{branch::alt, character::complete::digit1, combinator::eof, IResult, Parser};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::parse_separated_terminated,
    tag::complete::tag,
    ParserExt,
};

fn parse_meal(input: &str) -> IResult<&str, i64, ErrorTree<&str>> {
    digit1.parse_from_str().parse(input)
}

trait ElfCollect {
    fn new() -> Self;
    fn add(&mut self, meal: i64);
}

fn parse_elf<T: ElfCollect>(input: &str) -> IResult<&str, T, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_meal.context("meal"),
        tag("\n"),
        alt((eof.value(()), tag("\n\n").value(()))).peek(),
        T::new,
        |mut elf, meal| {
            elf.add(meal);
            elf
        },
    )
    .parse(input)
}

trait ElfSet {
    type Elf;

    fn new() -> Self;
    fn add(&mut self, elf: Self::Elf);
}

fn parse_elves<T: ElfSet>(input: &str) -> IResult<&str, T, ErrorTree<&str>>
where
    T::Elf: ElfCollect,
{
    parse_separated_terminated(
        parse_elf.context("elf"),
        tag("\n\n"),
        eof,
        T::new,
        |mut set, elf| {
            set.add(elf);
            set
        },
    )
    .parse(input)
}

fn final_parse_elves<T: ElfSet>(input: &str) -> Result<T, ErrorTree<Location>>
where
    T::Elf: ElfCollect,
{
    final_parser(parse_elves)(input)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
struct ElfTotal {
    total: i64,
}

impl ElfCollect for ElfTotal {
    fn new() -> Self {
        Self::default()
    }

    fn add(&mut self, meal: i64) {
        self.total += meal;
    }
}

#[derive(Debug, Default)]
struct BestElf {
    elf: ElfTotal,
}

impl ElfSet for BestElf {
    type Elf = ElfTotal;

    fn new() -> Self {
        Self::default()
    }

    fn add(&mut self, elf: Self::Elf) {
        self.elf = max(self.elf, elf);
    }
}

pub fn part1(input: &str) -> anyhow::Result<i64> {
    final_parse_elves(input.trim())
        .context("failed to parse elf list")
        .map(|best: BestElf| best.elf.total)
}

#[derive(Debug, Default)]
struct Best3 {
    elves: [ElfTotal; 3],
}

impl ElfSet for Best3 {
    type Elf = ElfTotal;

    fn new() -> Self {
        Self::default()
    }

    fn add(&mut self, elf: Self::Elf) {
        if elf > self.elves[0] {
            self.elves[0] = elf;
            self.elves.sort_unstable();
        }
    }
}

pub fn part2(input: &str) -> anyhow::Result<i64> {
    final_parse_elves(input.trim())
        .context("failed to parse elf list")
        .map(|best: Best3| best.elves.iter().copied().map(|elf| elf.total).sum())
}
