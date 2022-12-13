use std::{
    collections::{BTreeMap, HashMap},
    convert::Infallible,
};

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

use crate::parser;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Item(i64);

#[derive(Debug, Clone, Copy)]
enum Operand {
    Input,
    Literal(i64),
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

#[derive(Debug, Clone, Copy)]
struct DivisibilityTest {
    divisor: i64,
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

fn parse_number(input: &str) -> IResult<&str, i64, ErrorTree<&str>> {
    digit1.parse_from_str_cut().parse(input)
}

fn parse_item_set(input: &str) -> IResult<&str, Vec<Item>, ErrorTree<&str>> {
    collect_separated_terminated(
        parse_number.map(Item).context("item"),
        tag(",").delimited_by(space0),
        line_ending,
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
        parse_monkey_line("Operation", parse_operation.context("operation"))
            .preceded_by(tag("new = "))
            .context("operation line"),
        // Test: divisible by 10
        parse_monkey_line("Test", parse_number.context("test divisor"))
            .map(|divisor| DivisibilityTest { divisor })
            .preceded_by(tag("divisible by "))
            .context("test line"),
        // If true: throw to monkey N
        parse_monkey_line("If true", parse_throw).context("if true line"),
        // If false: throw to monkey N
        parse_monkey_line("If true", parse_throw).context("if false line"),
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

pub fn part1(input: Input) -> anyhow::Result<Infallible> {
    anyhow::bail!("not implemented yet")
}

pub fn part2(input: Input) -> anyhow::Result<Infallible> {
    anyhow::bail!("not implemented yet")
}
