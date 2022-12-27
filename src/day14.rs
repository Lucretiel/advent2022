use anyhow::Context;
use gridly::{prelude::*, range::RangeError};
use gridly_grids::SparseGrid;
use nom::{
    character::complete::{char, digit1, line_ending, multispace0},
    combinator::{eof, success},
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location as ErrorLocation},
    multi::{parse_separated_terminated, parse_separated_terminated_res},
    tag::complete::tag,
    ParserExt,
};

fn parse_coordinate<T: LocationComponent>(input: &str) -> IResult<&str, T, ErrorTree<&str>> {
    digit1
        .opt_preceded_by(char('-'))
        .recognize()
        .parse_from_str_cut()
        .map(T::from)
        .parse(input)
}

fn parse_coords(input: &str) -> IResult<&str, Location, ErrorTree<&str>> {
    parse_coordinate
        .context("x")
        .terminated(char(','))
        .and(parse_coordinate.context("y"))
        .map(|(column, row)| Location { row, column })
        .parse(input)
}

struct Chain {
    root: Location,
    movements: Vec<(Direction, isize)>,
}

impl Chain {
    pub fn locations(&self) -> impl Iterator<Item = Location> + '_ {
        self.movements
            .iter()
            .copied()
            .scan(self.root, |root, (direction, distance)| {
                let local_root = *root;
                let locations = (0..distance)
                    .map(|distance| (distance + 1))
                    .map(move |distance| direction * distance)
                    .map(move |vector| local_root + vector);

                *root += direction * distance;
                Some(locations)
            })
            .flatten()
            .chain([self.root])
    }
}

impl Chain {
    pub fn new(root: Location) -> Self {
        Self {
            root,
            movements: Vec::new(),
        }
    }
}

fn parse_chain(input: &str) -> IResult<&str, Chain, ErrorTree<&str>> {
    let (input, root) = parse_coords.context("root").parse(input)?;

    parse_separated_terminated_res(
        parse_coords.context("node").preceded_by(tag(" -> ")),
        success(()),
        line_ending.or(eof),
        move || (Chain::new(root), root),
        |(mut chain, root), next| {
            let vector = next - root;
            let direction = vector
                .direction()
                .context("node wasn't in a straight line from the previous one")?;
            let distance = (next - root).manhattan_length();
            chain.movements.push((direction, distance));
            anyhow::Ok((chain, next))
        },
    )
    .map(|(chain, _)| chain)
    .context("tail")
    .parse(input)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Cell {
    #[default]
    Empty,
    Rock,
    Sand,
}

fn parse_grid(input: &str) -> IResult<&str, SparseGrid<Cell>, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_chain.context("wall"),
        success(()),
        multispace0.terminated(eof),
        || SparseGrid::new(Vector::zero()),
        |mut grid, chain| {
            chain.locations().for_each(|location| {
                grid.insert(location, Cell::Rock);
            });
            grid
        },
    )
    .parse(input)
}

pub struct Input {
    grid: SparseGrid<Cell>,
}

impl TryFrom<&str> for Input {
    type Error = ErrorTree<ErrorLocation>;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        final_parser(parse_grid.map(|grid| Input { grid }))(value)
    }
}

#[derive(Debug, Copy, Clone)]
enum SearchResult {
    Available(Location),
    Void(Location),
}

pub fn part1(input: Input) -> anyhow::Result<usize> {
    let mut grid = input.grid;
    let sand_start = Column(500) + Row(0);

    // Each iteration of this loop is the entire journey for one piece of sand
    loop {
        let mut sand = sand_start;

        let sand = loop {
            match [Down.as_vector(), Down + Left, Down + Right]
                .iter()
                .copied()
                .map(|direction| sand + direction)
                .find_map(|attempt| match grid.get(attempt) {
                    Ok(Cell::Empty) | Err(BoundsError::Row(RangeError::TooLow(_))) => {
                        Some(SearchResult::Available(attempt))
                    }
                    Ok(_) => None,
                    Err(_) => Some(SearchResult::Void(attempt)),
                }) {
                Some(SearchResult::Available(new_sand)) => sand = new_sand,
                Some(SearchResult::Void(_)) => {
                    return Ok(grid
                        .occupied_entries()
                        .filter(|&(_, &cell)| matches!(cell, Cell::Sand))
                        .count())
                }
                None => break sand,
            }
        };

        match grid.get_mut(sand).ok() {
            Some(slot) => *slot = Cell::Sand,

            None => anyhow::bail!("Sand fell to rest in an out of bounds location: {sand:?}"),
        }
    }
}

pub fn part2(input: Input) -> anyhow::Result<usize> {
    let mut grid = input.grid;
    // The actual floor is 1 row below this; this is the location where sand
    // will come to rest
    let floor = grid.outer_bound().row;
    let sand_start = Column(500) + Row(0);

    // Each iteration of this loop is the entire journey for one piece of sand
    loop {
        let mut sand = sand_start;

        loop {
            match [Down.as_vector(), Down + Left, Down + Right]
                .iter()
                .copied()
                .map(|direction| sand + direction)
                .find_map(|attempt| match grid.get(attempt) {
                    Ok(Cell::Empty) | Err(BoundsError::Row(RangeError::TooLow(_))) => {
                        Some(SearchResult::Available(attempt))
                    }
                    Ok(_) => None,
                    Err(_) => Some(SearchResult::Void(attempt)),
                }) {
                Some(SearchResult::Available(new_sand)) => sand = new_sand,
                Some(SearchResult::Void(new_sand)) => {
                    let sand = (new_sand.column, floor).as_location();
                    grid.insert(sand, Cell::Sand);
                    break;
                }
                None => {
                    grid.insert(sand, Cell::Sand);
                    if sand == sand_start {
                        return Ok(grid
                            .occupied_entries()
                            .filter(|&(_, &cell)| matches!(cell, Cell::Sand))
                            .count());
                    } else {
                        break;
                    }
                }
            }
        }
    }
}
