use anyhow::Context;
use nom::{branch::alt, combinator::eof, IResult, Parser};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::parse_separated_terminated,
    tag::complete::tag,
    ParserExt as _,
};

#[derive(Debug, Clone, Copy)]
enum Player {
    Opponent,
    Me,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sign {
    Rock,
    Paper,
    Scissors,
}

impl Sign {
    pub fn wins_against(self) -> Sign {
        match self {
            Rock => Scissors,
            Paper => Rock,
            Scissors => Paper,
        }
    }

    pub fn loses_against(self) -> Sign {
        match self {
            Rock => Paper,
            Paper => Scissors,
            Scissors => Rock,
        }
    }
}

use Sign::*;

use crate::express;

fn generic_rule<'a, T>(rules: [(&'static str, T); 3]) -> impl Parser<&'a str, T, ErrorTree<&'a str>>
where
    T: Copy,
{
    alt((
        tag(rules[0].0).value(rules[0].1),
        tag(rules[1].0).value(rules[1].1),
        tag(rules[2].0).value(rules[2].1),
    ))
}

fn sign_rule<'a>(
    [rock, paper, scissors]: [&'static str; 3],
) -> impl Parser<&'a str, Sign, ErrorTree<&'a str>> {
    generic_rule([(rock, Rock), (paper, Paper), (scissors, Scissors)])
}

fn parse_opponent(input: &str) -> IResult<&str, Sign, ErrorTree<&str>> {
    sign_rule(["A", "B", "C"]).parse(input)
}

fn parse_me(input: &str) -> IResult<&str, Sign, ErrorTree<&str>> {
    sign_rule(["X", "Y", "Z"]).parse(input)
}

#[derive(Debug, Clone, Copy)]
struct Match {
    opponent: Sign,
    me: Sign,
}

fn parse_match(input: &str) -> IResult<&str, Match, ErrorTree<&str>> {
    parse_opponent
        .context("opponent sign")
        .terminated(tag(" "))
        .and(parse_me.context("own sign"))
        .map(|(opponent, me)| Match { opponent, me })
        .parse(input)
}

impl Match {
    fn play(&self) -> Option<Player> {
        if self.me.wins_against() == self.opponent {
            Some(Player::Me)
        } else if self.opponent.wins_against() == self.me {
            Some(Player::Opponent)
        } else {
            None
        }
    }
}

trait Evaluator: Default {
    fn add_match(&mut self, game: Match);
}

fn parse_matches<'a, T: Evaluator>(
    parse_match: impl Parser<&'a str, Match, ErrorTree<&'a str>>,
) -> impl Parser<&'a str, T, ErrorTree<&'a str>> {
    parse_separated_terminated(
        parse_match,
        tag("\n"),
        eof,
        T::default,
        |evaluator, game| express!(evaluator.add_match(game)),
    )
}

fn final_parse_matches<T: Evaluator>(input: &str) -> Result<T, ErrorTree<Location>> {
    final_parser(parse_matches(parse_match.context("match")))(input.trim())
}

#[derive(Debug, Default)]
struct TotalScore {
    score: i64,
}

impl Evaluator for TotalScore {
    fn add_match(&mut self, game: Match) {
        self.score += match game.me {
            Rock => 1,
            Paper => 2,
            Scissors => 3,
        };

        self.score += match game.play() {
            Some(Player::Opponent) => 0,
            None => 3,
            Some(Player::Me) => 6,
        };
    }
}

pub fn part1(input: &str) -> anyhow::Result<i64> {
    final_parse_matches(input)
        .context("failed to parse input")
        .map(|outcome: TotalScore| outcome.score)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Win,
    Draw,
    Lose,
}

use Outcome::*;

fn parse_outcome(input: &str) -> IResult<&str, Outcome, ErrorTree<&str>> {
    generic_rule([("X", Lose), ("Y", Draw), ("Z", Win)]).parse(input)
}

fn parse_match_v2(input: &str) -> IResult<&str, Match, ErrorTree<&str>> {
    parse_opponent
        .context("opponent sign")
        .terminated(tag(" "))
        .and(parse_outcome.context("predetermined outcome"))
        .map(|(opponent, outcome)| Match {
            opponent,
            me: match outcome {
                Draw => opponent,
                Win => opponent.loses_against(),
                Lose => opponent.wins_against(),
            },
        })
        .parse(input)
}

fn final_parse_matches_v2<T: Evaluator>(input: &str) -> Result<T, ErrorTree<Location>> {
    final_parser(parse_matches(parse_match_v2.context("match")))(input.trim())
}

pub fn part2(input: &str) -> anyhow::Result<i64> {
    final_parse_matches_v2(input)
        .context("failed to parse input")
        .map(|outcome: TotalScore| outcome.score)
}
