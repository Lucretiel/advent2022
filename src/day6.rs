use anyhow::Context;

pub fn unique<T: Eq>(mut input: &[T]) -> bool {
    loop {
        input = match input {
            [] | [_] => break true,
            [a, tail @ ..] => match tail.iter().all(|b| a != b) {
                false => break false,
                true => tail,
            },
        };
    }
}

fn start_of_marker_idx<T: Eq>(input: &[T], width: usize) -> Option<usize> {
    input
        .windows(width)
        .position(|window| unique(window))
        .map(|idx| idx + width)
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
    start_of_marker_idx(input.as_bytes(), 4).context("No start-of-packet marker")
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    start_of_marker_idx(input.as_bytes(), 14).context("No start-of-message marker")
}
