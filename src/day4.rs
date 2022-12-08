use anyhow::Context;
use nom::{
    character::complete::{char, digit1, line_ending},
    combinator::eof,
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::parse_separated_terminated,
    ParserExt,
};

use crate::parser;

/// Range of locations with inclusive min and max
#[derive(Debug, Clone, Copy)]
struct Range {
    min: i64,
    max: i64,
}

impl Range {
    pub fn contains(&self, other: &Self) -> bool {
        self.min <= other.min && self.max >= other.max
    }

    pub fn overlaps_into(&self, other: &Self) -> bool {
        other.min <= self.min && self.min <= other.max
            || other.min <= self.max && self.max <= other.max
    }
}

fn parse_number(input: &str) -> IResult<&str, i64, ErrorTree<&str>> {
    digit1.parse_from_str_cut().parse(input)
}

fn parse_range(input: &str) -> IResult<&str, Range, ErrorTree<&str>> {
    parser! {
        parse_number.context("lower bound") => min,
        char('-'),
        parse_number.context("upper bound") => max;
        Range { min, max }
    }
    .verify(|range| range.min <= range.max)
    .parse(input)
}

#[derive(Debug, Clone, Copy)]
struct RangePair {
    first: Range,
    second: Range,
}

impl RangePair {
    /// Returns true if one range is fully contained within the other
    fn fully_contained(&self) -> bool {
        self.first.contains(&self.second) || self.second.contains(&self.first)
    }

    fn overlaps(&self) -> bool {
        self.first.overlaps_into(&self.second) || self.second.overlaps_into(&self.first)
    }
}

fn parse_pair(input: &str) -> IResult<&str, RangePair, ErrorTree<&str>> {
    parser! {
        parse_range.context("first elf") => first,
        char(','),
        parse_range.context("second range") => second;
        RangePair { first, second }
    }
    .parse(input)
}

fn count_pair_list_matching<'a>(
    filter: impl Fn(&RangePair) -> bool,
) -> impl Parser<&'a str, usize, ErrorTree<&'a str>> {
    parse_separated_terminated(
        parse_pair.context("range pair"),
        line_ending,
        eof.opt_preceded_by(line_ending),
        || 0,
        move |count, range| match filter(&range) {
            true => count + 1,
            false => count,
        },
    )
}

fn final_count_pair_list_matching(
    input: &str,
    filter: impl Fn(&RangePair) -> bool,
) -> Result<usize, ErrorTree<Location>> {
    final_parser(count_pair_list_matching(filter))(input)
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
    final_count_pair_list_matching(input, |range| range.fully_contained())
        .context("failed to parse input")
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    final_count_pair_list_matching(input, |range| range.overlaps()).context("failed to parse input")
}
