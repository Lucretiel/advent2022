use std::cmp::Reverse;

use anyhow::Context;
use nom::{branch::alt, character::complete::digit1, combinator::eof, IResult, Parser};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::{collect_separated_terminated, parse_separated_terminated},
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

fn parse_elves<T: ElfCollect>(input: &str) -> IResult<&str, Vec<T>, ErrorTree<&str>> {
    collect_separated_terminated(parse_elf.context("elf"), tag("\n\n"), eof).parse(input)
}

fn final_parse_elves<T: ElfCollect>(input: &str) -> Result<Vec<T>, ErrorTree<Location>> {
    final_parser(parse_elves)(input)
}

#[derive(Debug, Clone, Copy, Default)]
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

pub fn part1(input: &str) -> anyhow::Result<i64> {
    let elves: Vec<ElfTotal> =
        final_parse_elves(input.trim()).context("failed to parse elf list")?;

    elves
        .iter()
        .copied()
        .map(|elf| elf.total)
        .max()
        .context("no elves in the input")
}

pub fn part2(input: &str) -> anyhow::Result<i64> {
    let mut elves: Vec<ElfTotal> =
        final_parse_elves(input.trim()).context("failed to parse elf list")?;

    elves.sort_unstable_by_key(|elf| Reverse(elf.total));

    Ok(elves.iter().copied().take(3).map(|elf| elf.total).sum())
}
