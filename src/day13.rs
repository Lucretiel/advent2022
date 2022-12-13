use std::{cmp::Ordering, ops::ControlFlow};

use itertools::Itertools;
use nom::{
    branch::alt,
    character::complete::{char, digit1, line_ending, multispace0},
    combinator::eof,
    sequence::tuple,
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::collect_separated_terminated,
    ParserExt,
};

use crate::library::Definitely;

#[derive(Debug, Clone)]
enum Value {
    Number(i64),
    List(Vec<Value>),

    // it's not specified what happens if a marker-equivalent packet is
    // present in the input, so we use a separate variant to track it.
    Marker(i64),
}

impl PartialEq for Value {
    #[inline]
    #[must_use]
    fn eq(&self, other: &Self) -> bool {
        Ord::cmp(self, other) == Ordering::Equal
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    #[inline]
    #[must_use]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Value {
    #[inline]
    #[must_use]
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Value::Marker(lhs), rhs) => Ord::cmp(&Value::Number(*lhs), rhs),
            (lhs, Value::Marker(rhs)) => Ord::cmp(lhs, &Value::Number(*rhs)),

            (Value::Number(lhs), Value::Number(rhs)) => Ord::cmp(lhs, rhs),
            (Value::List(lhs), Value::List(rhs)) => Ord::cmp(lhs, rhs),
            (Value::Number(lhs), Value::List(rhs)) => {
                Ord::cmp([Value::Number(*lhs)].as_slice(), rhs.as_slice())
            }
            (Value::List(lhs), Value::Number(rhs)) => {
                Ord::cmp(lhs.as_slice(), [Value::Number(*rhs)].as_slice())
            }
        }
    }
}

fn parse_number(input: &str) -> IResult<&str, i64, ErrorTree<&str>> {
    digit1.parse_from_str_cut().parse(input)
}

fn parse_list(input: &str) -> IResult<&str, Vec<Value>, ErrorTree<&str>> {
    collect_separated_terminated(parse_value.context("value"), char(','), char(']'))
        .or(char(']').map(|_| Vec::new()))
        .cut()
        .preceded_by(char('['))
        .parse(input)
}

fn parse_value(input: &str) -> IResult<&str, Value, ErrorTree<&str>> {
    alt((
        parse_number.context("number").map(Value::Number),
        parse_list.context("list").map(Value::List),
    ))
    .parse(input)
}

fn parse_value_pair(input: &str) -> IResult<&str, [Value; 2], ErrorTree<&str>> {
    parse_value
        .context("value")
        .separated_array(line_ending)
        .parse(input)
}

pub struct Input {
    pairs: Vec<[Value; 2]>,
}

fn parse_input(input: &str) -> IResult<&str, Input, ErrorTree<&str>> {
    collect_separated_terminated(
        parse_value_pair.context("value pair"),
        tuple((line_ending, line_ending)),
        multispace0.terminated(eof),
    )
    .map(|pairs| Input { pairs })
    .parse(input)
}

impl TryFrom<&str> for Input {
    type Error = ErrorTree<Location>;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        final_parser(parse_input)(value)
    }
}

fn is_sorted<T: Ord>(iter: impl IntoIterator<Item = T>) -> bool {
    let mut iter = iter.into_iter();
    let Some(first) = iter.next() else {return true};

    iter.try_fold(first, |lhs, rhs| match lhs <= rhs {
        true => ControlFlow::Continue(rhs),
        false => ControlFlow::Break(()),
    })
    .is_continue()
}

pub fn part1(input: Input) -> Definitely<usize> {
    Ok(input
        .pairs
        .iter()
        .enumerate()
        .filter(|&(_, pair)| is_sorted(pair))
        .map(|(idx, _)| idx + 1)
        .sum())
}

pub fn part2(input: Input) -> Definitely<usize> {
    let mut all_packets = input
        .pairs
        .into_iter()
        .flatten()
        .chain([Value::Marker(2), Value::Marker(6)])
        .collect_vec();

    // Possible alternative: instead of sorting, dump the packets into a binary
    // heap. It's linear time to build a heap and log(n) to pop from it, so if
    // most of the packets are larger than the markers, we might save some time.
    all_packets.sort_unstable();

    Ok(all_packets
        .iter()
        .positions(|value| matches!(value, Value::Marker(2 | 6)))
        .map(|idx| idx + 1)
        .product())
}
