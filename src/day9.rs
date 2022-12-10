use std::{cmp::max, collections::HashSet, iter};

use gridly::prelude::*;
use nom::{
    branch::alt,
    character::complete::{char, digit1, line_ending, multispace0, space1},
    combinator::eof,
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree, final_parser::final_parser, multi::collect_separated_terminated, ParserExt,
};

use crate::{library::Definitely, parser};

struct Command {
    direction: Direction,
    distance: usize,
}

fn parse_direction(input: &str) -> IResult<&str, Direction, ErrorTree<&str>> {
    alt((
        char('U').value(Up),
        char('R').value(Right),
        char('D').value(Down),
        char('L').value(Left),
    ))
    .parse(input)
}

fn parse_command(input: &str) -> IResult<&str, Command, ErrorTree<&str>> {
    parser! {
        parse_direction.context("direction") => direction,
        space1,
        digit1.parse_from_str_cut().context("distance") => distance;
        Command { direction, distance }
    }
    .parse(input)
}

fn parse_command_list(input: &str) -> IResult<&str, Vec<Command>, ErrorTree<&str>> {
    collect_separated_terminated(
        parse_command.context("command"),
        line_ending,
        multispace0.terminated(eof),
    )
    .parse(input)
}

pub struct CommandList {
    commands: Vec<Command>,
}

impl TryFrom<&str> for CommandList {
    type Error = ErrorTree<nom_supreme::final_parser::Location>;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        final_parser(parse_command_list)(value).map(|commands| CommandList { commands })
    }
}

fn diamond_length(vector: Vector) -> isize {
    max(vector.rows.0.abs(), vector.columns.0.abs())
}

#[derive(Debug)]
struct Rope<const N: usize> {
    body: [Location; N],
}

impl<const N: usize> Rope<N> {
    pub fn new() -> Self {
        Self {
            body: [Location::zero(); N],
        }
    }

    pub fn move_head(&mut self, direction: Direction) {
        let mut body = self.body.iter_mut();
        let Some(head) = body.next() else { return };
        *head += direction;

        let mut prev = *head;
        for part in body {
            let mut vector = prev - *part;
            if diamond_length(vector) > 1 {
                vector.rows.0 = vector.rows.0.signum();
                vector.columns.0 = vector.columns.0.signum();
                *part += vector;
                prev = *part;
            } else {
                break;
            }
        }
    }

    pub fn tail(&self) -> Location {
        self.body
            .last()
            .copied()
            .expect("length must be at least 1")
    }
}

fn record_motion<const N: usize>(input: &CommandList) -> usize {
    let mut rope: Rope<N> = Rope::new();

    let locations: HashSet<Location> = input
        .commands
        .iter()
        .flat_map(|command| iter::repeat(command.direction).take(command.distance))
        .map(|movement| {
            rope.move_head(movement);
            rope.tail()
        })
        .chain([Location::zero()])
        .collect();

    locations.len()
}

pub fn part1(input: CommandList) -> Definitely<usize> {
    Ok(record_motion::<2>(&input))
}

pub fn part2(input: CommandList) -> Definitely<usize> {
    Ok(record_motion::<10>(&input))
}
