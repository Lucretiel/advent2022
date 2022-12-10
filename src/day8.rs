use std::collections::BTreeSet;

use anyhow::Context;
use gridly::{location::RowOrderedLocation, prelude::*};
use gridly_grids::VecGrid;

use crate::library::Definitely;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Tree {
    height: u8,
}

pub struct TreeMap {
    trees: VecGrid<Tree>,
}

impl TryFrom<&str> for TreeMap {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        VecGrid::new_from_rows(
            value
                .lines()
                .map(|row| row.as_bytes().iter().copied().map(|height| Tree { height })),
        )
        .map(|trees| TreeMap { trees })
        .context("failed to build map; were all the rows the same length?")
    }
}

// Known bug: will return
fn fetch_visible_interior<T: Ord + Copy>(
    view: impl IntoIterator<Item = (Location, T)>,
) -> impl Iterator<Item = Location> {
    let mut state = None;
    view.into_iter()
        .filter(move |&(_, tree)| match state {
            None => {
                state = Some(tree);
                true
            }
            Some(tallest) if tree > tallest => {
                state = Some(tree);
                true
            }
            Some(_) => false,
        })
        .map(|(loc, _)| loc)
}

fn visibility_from<T: LocationComponent + 'static>(
    grid: &impl Grid<Item = Tree>,
) -> impl Iterator<Item = Location> + '_ {
    grid.view::<T>().iter().flat_map(|line| {
        let mut line1 = line.iter_with_locations().peekable();

        let loc1 = line1.peek().map(|&(loc, _)| loc);
        let rest1 = fetch_visible_interior(line1);

        let mut line2 = line.iter_with_locations().rev().peekable();
        let loc2 = line2.peek().map(|&(loc, _)| loc);
        let rest2 = fetch_visible_interior(line2);

        [loc1, loc2].into_iter().flatten().chain(rest1).chain(rest2)
    })
}

pub fn part1(input: TreeMap) -> Definitely<usize> {
    let vis1 = visibility_from::<Row>(&input.trees);
    let vis2 = visibility_from::<Column>(&input.trees);
    let locations: BTreeSet<RowOrderedLocation> =
        vis1.chain(vis2).map(RowOrderedLocation::new).collect();
    Ok(locations.len())
}

pub fn part2(input: TreeMap) -> anyhow::Result<isize> {
    input
        .trees
        .rows()
        .iter()
        .flat_map(|row| row.iter_with_locations())
        // For each tree in the forest...
        .map(|(location, &root)| {
            EACH_DIRECTION
                .into_iter()
                // Count the number of trees in each direction
                .map(|direction| {
                    (1isize..)
                        // Find the distance at which the intercepting tree appears,
                        // or the edge of the map
                        .find_map(|distance| {
                            match input.trees.get(location + (direction * distance)) {
                                Ok(&tree) => (tree >= root).then_some(distance),
                                Err(_) => Some(distance - 1),
                            }
                        })
                        .expect("find_map is guaranteed to terminate")
                })
                // Find the product of the tree counts from all 4 directions
                .product()
        })
        .max()
        .context("there were no trees in the grid")
}
