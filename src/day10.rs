use std::fmt::Display;

use nom::{
    branch::alt,
    character::complete::{digit1, line_ending, multispace0},
    combinator::eof,
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::parse_separated_terminated,
    tag::complete::tag,
    ParserExt,
};

use crate::{express, library::Definitely};

#[derive(Debug, Clone, Copy)]
enum Command {
    Noop,
    Addx(i64),
}

fn parse_command(input: &str) -> IResult<&str, Command, ErrorTree<&str>> {
    alt((
        tag("noop").value(Command::Noop),
        digit1
            .opt_preceded_by(tag("-"))
            .recognize()
            .parse_from_str_cut()
            .map(Command::Addx)
            .context("value")
            .cut()
            .preceded_by(tag("addx ")),
    ))
    .parse(input)
}

fn parse_states(input: &str) -> IResult<&str, Vec<(usize, i64)>, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_command.context("command"),
        line_ending,
        multispace0.terminated(eof),
        || (Vec::new(), 0),
        |(states, padding), command| match command {
            Command::Noop => (states, padding + 1),
            Command::Addx(delta) => (express!(states.push((padding + 2, delta))), 0),
        },
    )
    .map(|(states, _)| states)
    // Update all the states to contain absolute cycle counts rather than relative
    .map(|mut states| {
        let mut cycle = 0;

        states
            .iter_mut()
            .map(|&mut (ref mut delta, _)| delta)
            .for_each(move |delta| {
                cycle += *delta;
                *delta = cycle;
            });

        states
    })
    .parse(input)
}

pub struct States {
    states: Vec<(usize, i64)>,
}

impl TryFrom<&str> for States {
    type Error = ErrorTree<Location>;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        final_parser(parse_states)(input).map(|states| States { states })
    }
}

fn measure_signals(
    states: impl IntoIterator<Item = (usize, i64)>,
    targets: impl IntoIterator<Item = usize>,
) -> i64 {
    let mut register: i64 = 1;
    let mut total_signal: i64 = 0;

    let mut targets = targets.into_iter().peekable();

    states.into_iter().for_each(|(cycle, delta)| {
        while let Some(target_cycle) = targets.next_if(|&target_cycle| target_cycle < cycle) {
            eprintln!("register at {target_cycle} is {register}");
            total_signal += target_cycle as i64 * register;
        }

        register += delta;
    });

    targets
        .map(|target_cycle| target_cycle as i64 * register)
        .for_each(|signal| total_signal += signal);

    total_signal
}

pub fn part1(input: States) -> Definitely<i64> {
    Ok(measure_signals(
        input.states.iter().copied(),
        (0..).map(|i| i * 40).map(|i| i + 20).take(6),
    ))
}

struct Sprite {
    position: i64,
}

impl Sprite {
    fn new() -> Self {
        Self { position: 1 }
    }

    fn apply_move(&mut self, amount: i64) {
        self.position += amount
    }

    fn matches(&self, target: i64) -> bool {
        self.position >= target - 1 && self.position <= target + 1
    }
}

pub fn part2(input: States) -> Definitely<impl Display> {
    Ok(lazy_format::make_lazy_format!(|fmt| {
        let mut states = input.states.iter().copied().peekable();
        let mut sprite = Sprite::new();

        for line in 0..6 {
            for cell in 0..40 {
                let cycle = (line * 40) + cell;

                while let Some((_, command)) =
                    states.next_if(|(command_cycle, _)| *command_cycle <= cycle)
                {
                    sprite.apply_move(command);
                }

                if sprite.matches(cell as i64) {
                    write!(fmt, "#")?
                } else {
                    write!(fmt, " ")?
                }
            }

            write!(fmt, "\n")?
        }

        Ok(())
    }))
}
