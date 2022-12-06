use std::{collections::BTreeMap, fmt::Display};

use anyhow::Context;
use lazy_format::lazy_format;
use nom::{
    branch::alt,
    character::complete::{char, digit1, satisfy},
    combinator::{eof, success},
    error::ParseError,
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::collect_separated_terminated,
    tag::complete::tag,
    ParserExt,
};

use crate::parser;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Crate {
    id: char,
}

impl Crate {
    fn new(id: char) -> Self {
        Self { id }
    }
}

fn parse_crate(input: &str) -> IResult<&str, Crate, ErrorTree<&str>> {
    satisfy(|c| c.is_alphabetic())
        .map(Crate::new)
        .context("crate id")
        .preceded_by(char('['))
        .terminated(char(']'))
        .parse(input)
}

/// Parse a crate, or the absence of a crate
fn parse_crate_spot(input: &str) -> IResult<&str, Option<Crate>, ErrorTree<&str>> {
    alt((
        parse_crate.context("crate").map(Some),
        tag("   ").context("empty air").value(None),
    ))
    .parse(input)
}

fn parse_crate_row(input: &str) -> IResult<&str, Vec<Option<Crate>>, ErrorTree<&str>> {
    collect_separated_terminated(
        parse_crate_spot.context("crate row slot"),
        char(' '),
        char('\n'),
    )
    .parse(input)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct StackLabel<'a> {
    label: &'a str,
}

fn parse_stack_label(input: &str) -> IResult<&str, StackLabel<'_>, ErrorTree<&str>> {
    digit1.map(|label| StackLabel { label }).parse(input)
}

fn parse_stack_label_row<'a>(
    min_len: usize,
) -> impl Parser<&'a str, Vec<StackLabel<'a>>, ErrorTree<&'a str>> {
    collect_separated_terminated(
        parse_stack_label
            .context("stack label")
            .delimited_by(char(' '))
            .context("stack"),
        char(' '),
        char('\n'),
    )
    .map_res(move |labels: Vec<StackLabel<'_>>| {
        let len = labels.len();

        anyhow::ensure!(
            len >= min_len,
            "need at least {min_len} stack labels, but only got {len}"
        );

        Ok(labels)
    })
}

struct Stacks<'a> {
    stacks: BTreeMap<StackLabel<'a>, Vec<Crate>>,
}

fn parse_stacks(mut input: &str) -> IResult<&str, Stacks<'_>, ErrorTree<&str>> {
    let mut rows: Vec<Vec<Option<Crate>>> = Vec::new();

    let (input, labels) = loop {
        let longest_row_len = rows.iter().map(|row| row.len()).max().unwrap_or(0);

        let labels_err = match parse_stack_label_row(longest_row_len)
            .context("stack labels row")
            .parse(input)
        {
            Ok((tail, labels)) => break (tail, labels),
            Err(nom::Err::Error(err)) => err,
            Err(err) => return Err(err),
        };

        match parse_crate_row.context("crates row").parse(input) {
            Ok((tail, crates)) => {
                rows.push(crates);
                input = tail;
            }
            Err(err) => {
                return Err(match err {
                    nom::Err::Error(err) => nom::Err::Error(err.or(labels_err)),
                    err => err,
                })
            }
        }
    };

    let stacks = labels
        .iter()
        .copied()
        .enumerate()
        .map(|(idx, stack_label)| {
            let stack = rows.iter().rev().filter_map(|row| *row.get(idx)?).collect();
            (stack_label, stack)
        })
        .collect();

    Ok((input, Stacks { stacks }))
}

#[derive(Debug, Clone, Copy)]
struct Command<'a> {
    count: usize,
    origin: StackLabel<'a>,
    destination: StackLabel<'a>,
}

impl<'a> Stacks<'a> {
    pub fn apply_move(
        &mut self,
        origin: StackLabel<'a>,
        destination: StackLabel<'a>,
    ) -> anyhow::Result<()> {
        let origin = self
            .stacks
            .get_mut(&origin)
            .context("origin stack doesn't exist")?;
        let moved_crate = origin.pop().context("origin stack is empty")?;
        let destination = self
            .stacks
            .get_mut(&destination)
            .context("destination stack doesn't exist")?;
        destination.push(moved_crate);

        Ok(())
    }

    pub fn apply_command(&mut self, command: &Command<'a>) -> anyhow::Result<()> {
        (0..command.count).try_for_each(|n| {
            self.apply_move(command.origin, command.destination)
                .context(lazy_format!("failed during move #{}", n + 1))
        })
    }
}

fn parse_command(input: &str) -> IResult<&str, Command<'_>, ErrorTree<&str>> {
    parser! {
        tag("move "),
        digit1.parse_from_str().context("count") => count,
        tag(" from "),
        parse_stack_label.context("origin") => origin,
        tag(" to "),
        parse_stack_label.context("destination") => destination,
        tag("\n");
        Command{count, origin, destination}
    }
    .parse(input)
}

fn parse_command_list(input: &str) -> IResult<&str, Vec<Command<'_>>, ErrorTree<&str>> {
    collect_separated_terminated(parse_command.context("command"), success(()), eof).parse(input)
}

fn parse_problem(input: &str) -> IResult<&str, (Stacks<'_>, Vec<Command<'_>>), ErrorTree<&str>> {
    parser! {
        parse_stacks.context("stacks") => stacks,
        char('\n'),
        parse_command_list.context("commands") => commands;
        (stacks, commands)
    }
    .parse(input)
}

fn final_parse_problem(input: &str) -> Result<(Stacks<'_>, Vec<Command<'_>>), ErrorTree<Location>> {
    final_parser(parse_problem)(input)
}

pub fn part1(input: &str) -> anyhow::Result<impl Display + '_> {
    let (mut stacks, commands) = final_parse_problem(input).context("failed to parse input")?;

    commands
        .iter()
        .enumerate()
        .try_for_each(|(idx, command)| {
            stacks
                .apply_command(command)
                .context(lazy_format!("failed to apply command #{}", idx + 1))
        })
        .context("error while applying commands")?;

    Ok(
        lazy_format!("{label}" for Crate{id: label} in stacks.stacks.values().filter_map(|stack| stack.last())),
    )
}

pub fn part2(_input: &str) -> anyhow::Result<i64> {
    anyhow::bail!("not implemented yet")
}
