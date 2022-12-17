use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
    mem,
    str::FromStr,
};

use anyhow::Context;
use lazy_format::lazy_format;
use nom::{
    branch::alt,
    character::complete::{digit1, line_ending, multispace0, space0, space1},
    combinator::eof,
    sequence::tuple,
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::{collect_separated_terminated, parse_separated_terminated},
    tag::complete::tag,
    ParserExt,
};

use crate::{library::Counter, parser};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Item(i128);

#[derive(Debug, Clone, Copy)]
enum Operand {
    Input,
    Literal(i128),
}

impl Operand {
    pub fn get(self, input: i128) -> i128 {
        match self {
            Operand::Input => input,
            Operand::Literal(value) => value,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Operator {
    Plus,
    Times,
}

#[derive(Debug, Clone, Copy)]
struct Operation {
    first: Operand,
    op: Operator,
    second: Operand,
}

impl Operation {
    fn apply(&self, input: i128) -> Option<i128> {
        let first = self.first.get(input);
        let second = self.second.get(input);

        match self.op {
            Operator::Plus => first.checked_add(second),
            Operator::Times => first.checked_mul(second),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct DivisibilityTest {
    divisor: i128,
}

impl DivisibilityTest {
    pub fn apply(self, input: i128) -> bool {
        input % self.divisor == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct MonkeyId(i64);

#[derive(Debug, Clone, Copy)]
struct ThrowPreference {
    if_true: MonkeyId,
    if_false: MonkeyId,
}

#[derive(Debug, Clone)]
struct MonkeySpec {
    operation: Operation,
    test: DivisibilityTest,
    preference: ThrowPreference,
}

fn parse_number<T: FromStr>(input: &str) -> IResult<&str, T, ErrorTree<&str>>
where
    T::Err: Error + Send + Sync + 'static,
{
    digit1.parse_from_str_cut().parse(input)
}

fn parse_item_set(input: &str) -> IResult<&str, Vec<Item>, ErrorTree<&str>> {
    collect_separated_terminated(
        parse_number.map(Item).context("item"),
        tag(",").delimited_by(space0),
        line_ending.peek(),
    )
    .parse(input)
}

fn parse_monkey_line<'a, T>(
    prefix: &'static str,
    mut payload: impl Parser<&'a str, T, ErrorTree<&'a str>>,
) -> impl Parser<&'a str, T, ErrorTree<&'a str>> {
    parser! {
        space1.context("line indent"),
        tag(prefix).context("line prefix"),
        tag(":").delimited_by(space0),
        payload => value,
        line_ending;
        value
    }
}

fn parse_operand(input: &str) -> IResult<&str, Operand, ErrorTree<&str>> {
    alt((
        tag("old").value(Operand::Input),
        parse_number.map(Operand::Literal),
    ))
    .parse(input)
}

fn parse_operation(input: &str) -> IResult<&str, Operation, ErrorTree<&str>> {
    parser! {
        parse_operand.context("first") => first,

        alt((
            tag("+").value(Operator::Plus),
            tag("*").value(Operator::Times),
        ))
        .context("operator")
        .delimited_by(space0) => op,

        parse_operand.context("second") => second;

        Operation { first, op, second }
    }
    .parse(input)
}

fn parse_throw(input: &str) -> IResult<&str, MonkeyId, ErrorTree<&str>> {
    parse_number
        .map(MonkeyId)
        .preceded_by(tag("throw to monkey "))
        .parse(input)
}

fn parse_monkey(input: &str) -> IResult<&str, (MonkeyId, MonkeySpec, Vec<Item>), ErrorTree<&str>> {
    tuple((
        // Monkey N:
        parse_number
            .map(MonkeyId)
            .context("id")
            .terminated(tag(":"))
            .terminated(line_ending)
            .preceded_by(tag("Monkey "))
            .context("header"),
        // Starting items: 1, 2, 3
        parse_monkey_line("Starting items", parse_item_set.context("item set"))
            .context("item line"),
        // Operation: new = old + s
        parse_monkey_line(
            "Operation",
            parse_operation
                .context("operation")
                .preceded_by(tag("new = ")),
        )
        .context("operation line"),
        // Test: divisible by 10
        parse_monkey_line(
            "Test",
            parse_number
                .context("test divisor")
                .preceded_by(tag("divisible by ")),
        )
        .map(|divisor| DivisibilityTest { divisor })
        .context("test line"),
        // If true: throw to monkey N
        parse_monkey_line("If true", parse_throw).context("if true line"),
        // If false: throw to monkey N
        parse_monkey_line("If false", parse_throw).context("if false line"),
    ))
    .map(|(id, items, operation, test, if_true, if_false)| {
        (
            id,
            MonkeySpec {
                operation,
                test,
                preference: ThrowPreference { if_true, if_false },
            },
            items,
        )
    })
    .parse(input)
}

#[derive(Debug, Clone, Default)]
pub struct Input {
    specs: BTreeMap<MonkeyId, MonkeySpec>,
    collections: HashMap<MonkeyId, Vec<Item>>,
}

fn parse_input(input: &str) -> IResult<&str, Input, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_monkey.context("monkey"),
        line_ending,
        multispace0.terminated(eof),
        Input::default,
        |mut input, (id, spec, items)| {
            input.specs.insert(id, spec);
            input.collections.insert(id, items);
            input
        },
    )
    .parse(input)
}

impl TryFrom<&str> for Input {
    type Error = ErrorTree<Location>;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        final_parser(parse_input)(value)
    }
}

#[inline]
fn simulate_monkeys(mut input: Input, rounds: usize, div: bool) -> anyhow::Result<usize> {
    let mut inspection_counts = Counter::with_capacity(input.specs.len());
    let factor: i128 = input
        .specs
        .values()
        .map(|spec| spec.test.divisor)
        .try_fold(1i128, |accum, value| accum.checked_mul(value))
        .context("overflow while calculating factor")?;

    for _ in 0..rounds {
        for (&id, spec) in &input.specs {
            let items = mem::take(
                input
                    .collections
                    .get_mut(&id)
                    .context(lazy_format!("Monkey '{id:?}' has no items"))?,
            );

            inspection_counts.add(id, items.len());

            for item in items {
                let item = spec.operation.apply(item.0).context("overflow detected")?;
                let item = if div { item / 3 } else { item % factor };
                let target = if spec.test.apply(item) {
                    spec.preference.if_true
                } else {
                    spec.preference.if_false
                };
                input
                    .collections
                    .get_mut(&target)
                    .context(lazy_format!("Target monkey {target:?} doesn't exist"))?
                    .push(Item(item));
            }
        }
    }

    let [(_, count1), (_, count2)] = inspection_counts
        .top()
        .context("less than 2 monkeys did any throwing")?;

    Ok(count1 * count2)
}

pub fn part1(input: Input) -> anyhow::Result<usize> {
    simulate_monkeys(input, 20, true)
}

pub fn part2(input: Input) -> anyhow::Result<usize> {
    simulate_monkeys(input, 10_000, false)
}
