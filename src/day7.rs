use std::collections::HashMap;

use anyhow::Context;
use nom::{
    branch::alt,
    bytes::complete::take_until1,
    character::complete::{char, digit1, space0, space1},
    combinator::{eof, success},
    error::{ErrorKind, FromExternalError, ParseError},
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::parse_separated_terminated,
    tag::complete::tag,
    ParserExt,
};

#[derive(Debug, Copy, Clone)]
struct File {
    size: usize,
}

#[derive(Debug, Clone, Default)]
struct Directory<'a> {
    entries: HashMap<&'a str, Node<'a>>,
}

impl Directory<'_> {
    pub fn size(&self) -> usize {
        self.entries.values().map(|node| node.size()).sum()
    }
}

impl<'a> Directory<'a> {
    pub fn add_file(&mut self, name: &'a str, size: usize) {
        self.entries.insert(name, Node::File(File { size }));
    }

    pub fn add_directory(&mut self, name: &'a str) {
        self.entries
            .entry(name)
            .and_modify(|node| match node {
                Node::File(_) => *node = Node::Directory(Directory::default()),
                Node::Directory(_) => {}
            })
            .or_insert_with(|| Node::Directory(Directory::default()));
    }
}

#[derive(Debug, Clone)]
enum Node<'a> {
    File(File),
    Directory(Directory<'a>),
}

impl Node<'_> {
    pub fn size(&self) -> usize {
        match self {
            Node::File(file) => file.size,
            Node::Directory(dir) => dir.size(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Destination<'a> {
    Root,
    Up,
    Directory(&'a str),
}

fn parse_command<'a, T>(
    command: &'static str,
    terminator: impl Parser<&'a str, T, ErrorTree<&'a str>>,
) -> impl Parser<&'a str, T, ErrorTree<&'a str>> {
    char('$')
        .terminated(space1)
        .terminated(tag(command).context("command"))
        .precedes(terminator)
        .terminated(space0)
        .terminated(tag("\n"))
}

fn parse_cd(input: &str) -> IResult<&str, Destination<'_>, ErrorTree<&str>> {
    parse_command(
        "cd",
        take_until1("\n")
            .context("cd destination")
            .preceded_by(space1),
    )
    .map(|argument: &str| match argument.trim() {
        "/" => Destination::Root,
        ".." => Destination::Up,
        directory => Destination::Directory(directory),
    })
    .parse(input)
}

fn parse_ls_command(input: &str) -> IResult<&str, (), ErrorTree<&str>> {
    parse_command("ls", success(())).parse(input)
}

#[derive(Debug, Clone, Copy)]
enum EntryKind {
    File(usize),
    Directory,
}

#[derive(Debug, Clone, Copy)]
struct Entry<'a> {
    name: &'a str,
    kind: EntryKind,
}

fn parse_entry(input: &str) -> IResult<&str, Entry<'_>, ErrorTree<&str>> {
    alt((
        tag("dir").value(EntryKind::Directory),
        digit1
            .parse_from_str_cut()
            .map(EntryKind::File)
            .context("size"),
    ))
    .terminated(space1)
    .and(take_until1("\n").context("name"))
    .map(|(kind, name)| Entry { name, kind })
    .parse(input)
}

fn parse_ls_output(input: &str) -> IResult<&str, Directory<'_>, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_entry.terminated(tag("\n")).context("entry"),
        success(()),
        alt((eof, tag("$"))).peek(),
        Directory::default,
        |mut entries, entry| {
            match entry.kind {
                EntryKind::Directory => entries.add_directory(entry.name),
                EntryKind::File(size) => entries.add_file(entry.name, size),
            }

            entries
        },
    )
    .parse(input)
}

fn parse_directory_from_instructions(
    mut input: &str,
) -> IResult<&str, Directory<'_>, ErrorTree<&str>> {
    let mut root = Directory::default();
    let mut path = Vec::new();

    let mut current_dir = &mut root;

    loop {
        let cd_err = match parse_cd.context("cd").parse(input) {
            Ok((tail, destination)) => {
                match destination {
                    Destination::Root => {
                        path.clear();
                        current_dir = &mut root;
                    }
                    Destination::Up => {
                        let _ = path.pop();
                        current_dir = path.iter().fold(&mut root, |dir, name| {
                            match dir.entries.get_mut(name) {
                                None => panic!(
                                    "directory {name:?} doesn't exist; this shouldn't happen here"
                                ),
                                Some(Node::File(_)) => {
                                    panic!("{name:?} is a file, not a directory")
                                }
                                Some(Node::Directory(dir)) => dir,
                            }
                        });
                    }
                    Destination::Directory(name) => {
                        path.push(name);
                        current_dir = match current_dir.entries.get_mut(name) {
                            None => {
                                return Err(nom::Err::Failure(ErrorTree::from_external_error(
                                    input,
                                    ErrorKind::MapRes,
                                    anyhow::anyhow!("directory {name} doesn't exist"),
                                )))
                            }
                            Some(Node::File(_)) => {
                                return Err(nom::Err::Failure(ErrorTree::from_external_error(
                                    input,
                                    ErrorKind::MapRes,
                                    anyhow::anyhow!("{name:?} is a file"),
                                )))
                            }
                            Some(Node::Directory(directory)) => directory,
                        };
                    }
                }

                input = tail;
                continue;
            }
            Err(nom::Err::Error(cd_err)) => cd_err,
            Err(err) => return Err(err),
        };

        let ls_err = match parse_ls_command
            .context("command")
            .precedes(parse_ls_output.context("output"))
            .context("ls")
            .parse(input)
        {
            Ok((tail, directory)) => {
                *current_dir = directory;
                input = tail;
                continue;
            }
            Err(nom::Err::Error(err)) => err,
            Err(err) => return Err(err),
        };

        return match eof.value(()).parse(input) {
            Ok((tail, ())) => Ok((tail, root)),
            Err(nom::Err::Error(err)) => Err(nom::Err::Error(cd_err.or(ls_err).or(err))),
            Err(err) => Err(err),
        };
    }
}

fn final_parse_directory_from_instructions(
    input: &str,
) -> Result<Directory<'_>, ErrorTree<Location>> {
    final_parser(parse_directory_from_instructions)(input)
}

fn weird_recursive_size(directory: &Directory) -> usize {
    let size = directory.size();
    let size = if size <= 100000 { size } else { 0 };

    directory
        .entries
        .values()
        .filter_map(|node| match node {
            Node::Directory(dir) => Some(dir),
            Node::File(_) => None,
        })
        .map(weird_recursive_size)
        .sum::<usize>()
        + size
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
    let directory =
        final_parse_directory_from_instructions(input).context("failed to parse input")?;

    Ok(weird_recursive_size(&directory))
}

fn walk_directories<'a, 'n>(
    name: &'n str,
    root: &'a Directory<'n>,
    scan: &mut impl FnMut(&'n str, &'a Directory<'n>),
) {
    scan(name, root);
    root.entries
        .iter()
        .filter_map(|(name, node)| match node {
            Node::Directory(dir) => Some((name, dir)),
            Node::File(_) => None,
        })
        .for_each(|(name, dir)| walk_directories(name, dir, scan))
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    let directory =
        final_parse_directory_from_instructions(input).context("failed to parse input")?;

    let total_space = 70_000_000;
    let used_space = directory.size();
    eprintln!("Used: {used_space}");
    let unused_space = total_space - used_space;

    eprintln!("Unused: {unused_space}");
    let required_space = 30_000_000;
    let min_deletion = required_space - unused_space;
    eprintln!("Min deletion: {min_deletion}");

    let mut best_dir = None;

    walk_directories("/", &directory, &mut |_, dir| {
        let size = dir.size();

        if size >= min_deletion {
            match best_dir {
                None => best_dir = Some(size),
                Some(best) if size < best => best_dir = Some(size),
                Some(_) => {}
            }
        }
    });

    best_dir.context("No directory was large enough to delete")
}
