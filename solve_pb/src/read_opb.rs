use std::{io::BufRead, str::FromStr};

use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::tag,
    character::{
        complete::{digit1, newline, not_line_ending, space1},
        streaming::space0,
    },
    combinator::{map, map_res, opt, recognize},
    multi::many1,
};
use num::{Integer, Signed};

#[derive(Clone, Debug)]
pub struct PBProblem {
    pub constraints: Vec<Constraint>,
}

#[derive(Clone, Debug)]
pub enum CommentOrConstraint {
    Comment(String),
    Constraint(Constraint),
}

#[derive(Clone, Debug)]
pub struct Constraint {
    pub sum: Vec<WeightedTerm>,
    pub relational_operator: RelationalOperator,
    pub rhs: i64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RelationalOperator {
    Equal,
    GreaterOrEqual,
}

#[derive(Clone, Debug)]
pub struct WeightedTerm {
    pub weight: i64,
    pub term: Term,
}

pub type Term = Variable;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Variable {
    pub index: usize,
}

pub fn read_opb(
    input: &mut std::io::BufReader<impl std::io::Read>,
) -> Option<PBProblem> {
    let mut constraints = Vec::default();

    let mut line = String::default();
    loop {
        line.clear();
        let Ok(bytes) = input.read_line(&mut line) else {
            return None;
        };
        if bytes == 0 {
            break;
        }
        let Ok((residual, comment_or_constraint)) = comment_or_constraint(line.as_str()) else {
            return None;
        };
        if residual != "" {
            return None;
        }

        if let CommentOrConstraint::Constraint(constraint) = comment_or_constraint {
            constraints.push(constraint);
        }
    }

    // PBProblem を構築して返す
    return Some(PBProblem { constraints });
}

fn sequence_of_comment_or_constraint(input: &str) -> IResult<&str, Vec<CommentOrConstraint>> {
    // <sequence_of_comments_or_constraints> ::= <comment_or_constraint> [<sequence_of_comments_or_constraints>]
    many1(comment_or_constraint).parse(input)
}

fn comment_or_constraint(input: &str) -> IResult<&str, CommentOrConstraint> {
    // <comment_or_constraint> ::= <comment> | <constraint>
    alt((
        map(comment, CommentOrConstraint::Comment),
        map(constraint, CommentOrConstraint::Constraint),
    ))
    .parse(input)
}

fn comment(input: &str) -> IResult<&str, String> {
    // <comment> ::= "*" <any_sequence_of_characters_other_than_EOL> <EOL>
    map((tag("*"), not_line_ending, newline), |(_, comment, _)| {
        str::to_string(comment)
    })
    .parse(input)
}

fn constraint(input: &str) -> IResult<&str, Constraint> {
    // <constraint>::= <sum> <relational_operator> <zeroOrMoreSpace> <integer> <zeroOrMoreSpace> ";"
    // ↑おそらく定義のミスで，実際のデータでは";"の後に改行がある
    map(
        (
            sum,
            relational_operator,
            space0,
            integer,
            space0,
            tag(";"),
            opt(newline),
        ),
        |(sum, relational_operator, _, rhs, _, _, _)| Constraint {
            sum,
            relational_operator,
            rhs,
        },
    )
    .parse(input)
}

fn relational_operator(input: &str) -> IResult<&str, RelationalOperator> {
    // <relational_operator> ::= "=" | ">="
    alt((
        map(tag("="), |_| RelationalOperator::Equal),
        map(tag(">="), |_| RelationalOperator::GreaterOrEqual),
    ))
    .parse(input)
}

fn sum(input: &str) -> IResult<&str, Vec<WeightedTerm>> {
    // <sum> ::= <weightedterm> | <weightedterm> <sum>
    many1(weighted_term).parse(input)
}

fn weighted_term(input: &str) -> IResult<&str, WeightedTerm> {
    // <weightedterm> ::= <integer> <oneOrMoreSpace> <term> <oneOrMoreSpace>
    // <term>::=<variableName>  # for linear instances
    map(
        (integer, space1, variable_name, space1),
        |(weight, _, term, _)| WeightedTerm { weight, term },
    )
    .parse(input)
}

fn variable_name(input: &str) -> IResult<&str, Variable> {
    // <variableName> ::= "x" <unsigned_integer>
    map((tag("x"), unsigined_integer), |(_, index)| Variable {
        index,
    })
    .parse(input)
}

fn integer<IntT: Signed + FromStr>(input: &str) -> IResult<&str, IntT> {
    // <integer> ::= <unsigned_integer> | "+" <unsigned_integer> | "-" <unsigned_integer>
    map_res(
        alt((
            digit1,
            recognize((tag("+"), digit1)),
            recognize((tag("-"), digit1)),
        )),
        str::parse,
    )
    .parse(input)
}

fn unsigined_integer<UIntT: Integer + FromStr>(input: &str) -> IResult<&str, UIntT> {
    // <unsigned_integer> ::= <digit> | <digit><unsigned_integer>
    map_res(digit1, str::parse).parse(input)
}

// 別の実装

#[cfg(test)]
fn integer1<IntT: Signed + FromStr>(input: &str) -> IResult<&str, IntT> {
    // integer の別の実装 1
    // <integer> ::= <unsigned_integer> | "+" <unsigned_integer> | "-" <unsigned_integer>
    alt((
        map_res(digit1, str::parse),
        map_res(recognize((tag("+"), digit1)), str::parse),
        map_res(recognize((tag("-"), digit1)), str::parse),
    ))
    .parse(input)
}

#[cfg(test)]
fn integer2<IntT: Signed + FromStr>(input: &str) -> IResult<&str, IntT> {
    // integer の別の実装 2
    // <integer> ::= ["+" | "-"] <unsigned_integer>
    map_res(
        recognize((opt(alt((tag("+"), tag("-")))), digit1)),
        str::parse,
    )
    .parse(input)
}

#[cfg(test)]
mod test {

    use std::{fmt::Debug, str::FromStr};

    use nom::IResult;
    use num::{Integer, Signed};

    use crate::read_opb::{integer1, unsigined_integer};

    use super::{Variable, integer, integer2, read_opb};

    #[test]
    fn test_unsigined_integer() {
        fn run<UIntT: Integer + FromStr + Copy + Debug + TryFrom<i32>>(
            integer: impl Fn(&str) -> IResult<&str, UIntT>,
        ) where
            <UIntT as TryFrom<i32>>::Error: Debug,
        {
            let v = 123.try_into().unwrap();
            assert_eq!(integer("123"), Ok(("", v)));
            assert_eq!(integer("123x"), Ok(("x", v)));
            assert_eq!(integer("123.45"), Ok((".45", v)));

            assert!(integer("").is_err());
            assert!(integer("+").is_err());
            assert!(integer("-").is_err());
            assert!(integer("+123").is_err());
            assert!(integer("-123").is_err());
            assert!(integer("+ 123").is_err());
            assert!(integer("- 123").is_err());
        }

        run(unsigined_integer::<i64>);
        run(unsigined_integer::<u64>);
        run(unsigined_integer::<isize>);
        run(unsigined_integer::<usize>);
    }

    #[test]
    fn test_integer() {
        fn run<IntT: Signed + FromStr + Copy + Debug + TryFrom<i32>>(
            integer: impl Fn(&str) -> IResult<&str, IntT>,
        ) where
            <IntT as TryFrom<i32>>::Error: Debug,
        {
            let v = 123.try_into().unwrap();
            assert_eq!(integer("123"), Ok(("", v)));
            assert_eq!(integer("123x"), Ok(("x", v)));
            assert_eq!(integer("+123"), Ok(("", v)));
            assert_eq!(integer("-123"), Ok(("", -v)));
            assert_eq!(integer("123.45"), Ok((".45", v)));

            assert!(integer("").is_err());
            assert!(integer("+").is_err());
            assert!(integer("-").is_err());
            assert!(integer("+ 123").is_err());
            assert!(integer("- 123").is_err());
        }

        run(integer::<i64>);
        run(integer::<isize>);
        run(integer1::<i64>);
        run(integer1::<isize>);
        run(integer2::<i64>);
        run(integer2::<isize>);
    }

    #[test]
    fn test_variable() {
        use super::variable_name;
        assert_eq!(variable_name("x1"), Ok(("", Variable { index: 1 })));
        assert_eq!(
            variable_name("x123456789"),
            Ok(("", Variable { index: 123456789 }))
        );
        assert!(variable_name("").is_err());
        assert!(variable_name("12").is_err());
        assert!(variable_name("y123").is_err());
        assert!(variable_name("x").is_err());
    }

    #[test]
    fn test_weighted_term() {
        use super::weighted_term;

        let (s, t) = weighted_term("3 x1 ").unwrap();
        assert!(s == "");
        assert!(t.weight == 3);
        assert!(t.term.index == 1);

        let (s, t) = weighted_term("-3 x1 ").unwrap();
        assert!(s == "");
        assert!(t.weight == -3);
        assert!(t.term.index == 1);

        assert!(weighted_term("3 x1").is_err());
        assert!(weighted_term("3x1").is_err());
        assert!(weighted_term("x1 ").is_err());
        assert!(weighted_term("x1").is_err());
    }

    //     #[test]
    //     fn test_opb() {
    //         let input = r"* comment
    // 1 x1 +1 x2 >= 1 ;
    // -1 x2 +1 x4 +1 x5 >= 0 ;
    // 1 x3 -1 x4 +1 x5 >= 0 ;
    // 1 x4 -1 x5 >= 0 ;
    // -1 x4 -1 x5 >= -1 ;
    // ";
    //         let result = read_opb(input);
    //         eprintln!("{:?}", result);
    //         assert!(result.is_ok());
    //     }
}
