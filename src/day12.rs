use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
};

use anyhow::Context;
use gridly::prelude::*;
use gridly_grids::VecGrid;
use itertools::Itertools;
use lazy_format::lazy_format;

#[derive(Debug, Copy, Clone)]
pub enum Site {
    Start,
    End,
    Normal(u8),
}

impl Site {
    fn new(value: char) -> anyhow::Result<Self> {
        match value as u8 {
            b'S' => Ok(Self::Start),
            b'E' => Ok(Self::End),
            value @ b'a'..=b'z' => Ok(Self::Normal(value - b'a')),
            _ => Err(anyhow::anyhow!("invalid site character")),
        }
    }

    fn height(self) -> u8 {
        match self {
            Site::Start => 0,
            Site::End => 25,
            Site::Normal(height) => height,
        }
    }
}

pub struct Input {
    grid: VecGrid<Site>,
    origin: Location,
    destination: Location,
}

impl TryFrom<&str> for Input {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let rows: Vec<Vec<Site>> = value
            .lines()
            .enumerate()
            .map(|(row_idx, row)| {
                row.chars()
                    .enumerate()
                    .map(|(col_index, cell)| {
                        Site::new(cell).context(lazy_format!(
                            "invalid cell {cell:?} at row {row_idx}, column {col_index}"
                        ))
                    })
                    .try_collect()
            })
            .try_collect()?;

        let grid = VecGrid::new_from_rows(rows).context("failed to build grid out of rows")?;

        let origin = grid
            .rows()
            .iter()
            .flat_map(|row| row.iter_with_locations())
            .find(|&(_, &site)| matches!(site, Site::Start))
            .map(|(location, _)| location)
            .context("no start site in grid")?;

        let destination = grid
            .rows()
            .iter()
            .flat_map(|row| row.iter_with_locations())
            .find(|&(_, &site)| matches!(site, Site::End))
            .map(|(location, _)| location)
            .context("no start site in grid")?;

        Ok(Input {
            grid,
            origin,
            destination,
        })
    }
}

fn count_steps(
    input: Input,
    build_initial_frontier: impl FnOnce(&Input) -> HashMap<Location, Site>,
) -> anyhow::Result<usize> {
    let mut seen = HashSet::new();
    let mut frontier = build_initial_frontier(&input);

    for steps in 0.. {
        seen.extend(frontier.keys().copied());
        let mut new_frontier = HashMap::with_capacity(frontier.len());

        for (&loc, &site) in &frontier {
            if matches!(site, Site::End) {
                return Ok(steps);
            }

            for direction in EACH_DIRECTION {
                let next_loc = loc + direction;
                let Ok(&next_site) = input.grid.get(next_loc) else { continue };
                if !seen.contains(&next_loc) {
                    if next_site.height() <= site.height() + 1 {
                        new_frontier.insert(next_loc, next_site);
                    }
                }
            }
        }

        frontier = new_frontier
    }

    anyhow::bail!("no path to end")
}

pub fn part1(input: Input) -> anyhow::Result<usize> {
    count_steps(input, |input| HashMap::from([(input.origin, Site::Start)]))
}

pub fn part2(input: Input) -> anyhow::Result<usize> {
    count_steps(input, |input| {
        input
            .grid
            .rows()
            .iter()
            .flat_map(|row| row.iter_with_locations())
            .map(|(loc, &site)| (loc, site))
            .filter(|&(_, site)| site.height() == 0)
            .collect()
    })
}
