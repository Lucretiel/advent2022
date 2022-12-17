use anyhow::Context;
use gridly::prelude::*;
use nom::{
    character::complete::{char, digit1, line_ending, multispace0},
    combinator::eof,
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree, final_parser::final_parser, multi::collect_separated_terminated,
    tag::complete::tag, ParserExt,
};
use rayon::prelude::*;

use crate::parser;

pub struct Input {
    signals: Vec<Signal>,
}

fn parse_component<'a, T: LocationComponent>(
    prefix: &'static str,
) -> impl Parser<&'a str, T, ErrorTree<&'a str>> {
    digit1
        .opt_preceded_by(char('-'))
        .recognize()
        .parse_from_str_cut()
        .map(|coord: isize| T::from(coord))
        .preceded_by(char('='))
        .preceded_by(tag(prefix))
}

fn parse_location(input: &str) -> IResult<&str, Location, ErrorTree<&str>> {
    parse_component("x")
        .context("column")
        .terminated(tag(", "))
        .and(parse_component("y").context("row"))
        .map(|(column, row)| Location { row, column })
        .parse(input)
}

#[derive(Debug, Clone, Copy)]
struct Signal {
    sensor: Location,
    beacon: Location,
}

impl Signal {
    fn radius(&self) -> isize {
        (self.beacon - self.sensor).manhattan_length()
    }
}

fn parse_signal(input: &str) -> IResult<&str, Signal, ErrorTree<&str>> {
    parser! {
        tag("Sensor at "),
        parse_location.context("sensor") => sensor,
        tag(": closest beacon is at "),
        parse_location.context("beacon") => beacon;
        Signal { sensor, beacon }
    }
    .parse(input)
}

fn parse_signals(input: &str) -> IResult<&str, Vec<Signal>, ErrorTree<&str>> {
    collect_separated_terminated(
        parse_signal.context("signal"),
        line_ending,
        multispace0.terminated(eof),
    )
    .parse(input)
}

impl TryFrom<&str> for Input {
    type Error = ErrorTree<nom_supreme::final_parser::Location>;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        final_parser(parse_signals)(value).map(|signals| Input { signals })
    }
}

pub fn part1(input: Input) -> anyhow::Result<usize> {
    // Need to determine our scanning distance. Find the leftmost and rightmost
    // sensors and add their respective radii.
    let start: Column = input
        .signals
        .iter()
        .map(|signal| signal.sensor.column)
        .min()
        .context("no signals in the input")?;

    let end: Column = input
        .signals
        .iter()
        .map(|signal| signal.sensor.column)
        .max()
        .context("no signals in the input")?;

    let radius = input
        .signals
        .iter()
        .map(|signal| signal.radius())
        .max()
        .context("no signals in the input")?;

    // To prevent tiny off by one errors, add a small buffer to both sides
    let start = start - Columns(radius + 10);
    let end = end + Columns(radius + 10);

    eprintln!("Identified start and end: {start:?} .. {end:?}");

    let row = Row(2000000);

    // TODO: this is embarrassingly parallel, get rayon in here to help out
    let in_range_count = (start.0..end.0)
        .into_par_iter()
        .map(|column| Column(column))
        .map(|column| column + row)
        .filter(|&location| {
            input
                .signals
                .iter()
                .any(|signal| (location - signal.sensor).manhattan_length() <= signal.radius())
        })
        .filter(|&location| input.signals.iter().all(|signal| signal.beacon != location))
        .count();

    Ok(in_range_count)
}

pub fn part2(input: Input) -> anyhow::Result<isize> {
    // Basic idea: we're guaranteed that there is only one possible location.
    // This means that it lies on the edge of one of the beacons, so search
    // the perimeters of each beacon
    input
        .signals
        .par_iter()
        .flat_map(|signal| {
            let radius = signal.radius() + 1;
            (0..radius)
                .into_par_iter()
                // Compute vectors resembling (4, 0), (3, 1), (2, 2), (1, 3)
                .map(move |delta| Vector {
                    rows: Rows(delta),
                    columns: Columns(radius - delta),
                })
                // Get all 4 rotations of that vector
                .flat_map_iter(|vector| {
                    [
                        vector,
                        vector.clockwise(),
                        vector.anticlockwise(),
                        vector.reverse(),
                    ]
                })
                // Add to the sensor to find the perimeter locations
                .map(move |vector| signal.sensor + vector)
        })
        .filter(|location| {
            0 <= location.row.0
                && location.row.0 <= 4_000_000
                && 0 <= location.column.0
                && location.column.0 <= 4_000_000
        })
        .find_any(|&location| {
            input
                .signals
                .iter()
                .all(|signal| (location - signal.sensor).manhattan_length() > signal.radius())
        })
        .context("no available beacon location")
        .map(|beacon| beacon.column.0 * 4_000_000 + beacon.row.0)
}
