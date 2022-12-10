use std::convert::Infallible;

pub struct Input;

impl TryFrom<&str> for Input {
    type Error = Infallible;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Self)
    }
}

pub fn part1(input: Input) -> anyhow::Result<Infallible> {
    anyhow::bail!("not implemented yet")
}

pub fn part2(input: Input) -> anyhow::Result<Infallible> {
    anyhow::bail!("not implemented yet")
}
