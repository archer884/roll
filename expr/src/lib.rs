use std::cmp;

mod error;

pub use error::Error;
use regex::Regex;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub struct ExpressionParser {
    bounded_expression: Regex,
    modifier_expression: Regex,
    reroll_expression: Regex,
    explode_expression: Regex,
}

impl ExpressionParser {
    pub fn new() -> Self {
        ExpressionParser {
            bounded_expression: Regex::new(r#"^([Aa]|[Ss])?(\d+[Dd])?[Dd]?(\d+)"#).unwrap(),
            modifier_expression: Regex::new(r#"([+-]\d+)"#).unwrap(),
            reroll_expression: Regex::new(r#"r(\d+)?"#).unwrap(),
            explode_expression: Regex::new(r#"!(\d+)?"#).unwrap(),
        }
    }

    pub fn parse(&self, expr: &str) -> Result<Expression> {
        let mut expression = Expression::default();

        match self.bounded_expression.captures(expr) {
            Some(captures) => {
                if let Some(group) = captures.get(1) {
                    expression.advantage = match group.as_str() {
                        "A" | "a" => Advantage::Advantage,
                        "S" | "s" => Advantage::Disadvantage,
                        _ => unreachable!("Regex can't match this"),
                    };
                }

                if let Some(group) = captures.get(2) {
                    let subexpr = group.as_str();
                    let subexpr = &subexpr[..subexpr.len() - 1];
                    expression.count = subexpr
                        .parse()
                        .map_err(|e| Error::BadInteger(subexpr.into(), e))?;
                } else {
                    expression.count = 1;
                }

                let max = captures
                    .get(3)
                    .ok_or_else(|| Error::BadExpression(expr.into()))?
                    .as_str();
                expression.max = max.parse().map_err(|e| Error::BadInteger(max.into(), e))?;
            }
            None => return Err(Error::BadExpression(expr.into())),
        }

        if let Some(text) = self.modifier_expression.find(expr) {
            expression.modifier = text
                .as_str()
                .parse()
                .map_err(|e| Error::BadInteger(text.as_str().into(), e))?;
        }

        expression.reroll = parse_threshold_token(expr, &self.reroll_expression, 1)?.map(Reroll);
        expression.explode =
            parse_threshold_token(expr, &self.explode_expression, expression.max)?.map(Explode);

        Ok(expression)
    }
}

impl Default for ExpressionParser {
    fn default() -> Self {
        ExpressionParser::new()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Expression {
    count: i32,
    max: i32,
    modifier: i32,
    advantage: Advantage,
    reroll: Option<Reroll>,
    explode: Option<Explode>,
}

impl Expression {
    fn reroll(&self, value: i32) -> bool {
        self.reroll
            .map(|x| x.should_reroll(value))
            .unwrap_or_default()
    }

    fn explode(&self, value: i32) -> bool {
        self.explode
            .map(|x| x.should_explode(value))
            .unwrap_or_default()
    }

    pub fn average_result(&self) -> f64 {
        ((1 + self.max) * self.count + self.modifier * 2) as f64 / 2.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Advantage {
    Advantage,
    Disadvantage,
    Normal,
}

impl Default for Advantage {
    fn default() -> Self {
        Advantage::Normal
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Reroll(i32);

impl Reroll {
    fn should_reroll(self, value: i32) -> bool {
        self.0 >= value
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Explode(i32);

impl Explode {
    fn should_explode(self, value: i32) -> bool {
        value >= self.0
    }
}

pub trait Realizer {
    fn next(&mut self, max: i32) -> i32;

    fn realize(&mut self, expression: &Expression) -> RealizedExpression {
        let mut results = SmallVec::new();
        let mut advantage = Some(expression.advantage);

        for _ in 0..expression.count {
            let mut value = match advantage.take().unwrap_or_default() {
                Advantage::Advantage => {
                    cmp::max(self.next(expression.max), self.next(expression.max))
                }
                Advantage::Disadvantage => {
                    cmp::min(self.next(expression.max), self.next(expression.max))
                }
                Advantage::Normal => self.next(expression.max),
            };

            loop {
                // If the value is small enough to re-roll, do not store it.
                if expression.reroll(value) {
                    value = self.next(expression.max);
                    continue;
                }

                // Store value.
                results.push(value);

                // If the value is large enough to explode, roll another and continue.
                if expression.explode(value) {
                    value = self.next(expression.max);
                    continue;
                }

                break;
            }
        }

        RealizedExpression {
            results,
            max: expression.max,
            modifier: expression.modifier,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RealizedExpression {
    results: SmallVec<[i32; 4]>,
    max: i32,
    modifier: i32,
}

impl RealizedExpression {
    pub fn sum(&self) -> i32 {
        let result: i32 = self.results.iter().sum();
        result + self.modifier
    }

    pub fn modifier(&self) -> i32 {
        self.modifier
    }

    pub fn results(&'_ self) -> impl Iterator<Item = (Highlight, i32)> + '_ {
        self.results.iter().map(move |&x| match x {
            1 => (Highlight::Low, 1),
            x if x == self.max => (Highlight::High, x),
            x => (Highlight::Normal, x),
        })
    }

    pub fn is_critical(&self) -> bool {
        self.results.len() == 1 && self.sum() == self.max + self.modifier
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Highlight {
    High,
    Low,
    Normal,
}

fn parse_threshold_token(expr: &str, pattern: &Regex, default: i32) -> Result<Option<i32>> {
    match pattern.captures(expr) {
        Some(captures) => match captures.get(1).map(|x| x.as_str()) {
            Some(text) => Ok(Some(
                text.parse()
                    .map_err(|e| Error::BadInteger(text.into(), e))?,
            )),
            None => Ok(Some(default)),
        },
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use crate::{Advantage, Explode, Expression, ExpressionParser, Realizer, Reroll};

    #[test]
    fn bounded_expression() {
        let expression = parse("2d6");
        assert_eq!(count_max(2, 6), expression);
    }

    #[test]
    fn leading_bounded_expression() {
        let a = parse("20");
        let b = parse("d20");
        let expected = count_max(1, 20);
        assert_eq!(a, expected);
        assert_eq!(b, expected);
    }

    #[test]
    fn bounded_expression_with_reroll() {
        let actual = parse("2d6r");
        let expected = Expression {
            count: 2,
            max: 6,
            reroll: Some(Reroll(1)),
            ..Default::default()
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn bounded_expression_with_reroll_2() {
        let actual = parse("2d6r2");
        let expected = Expression {
            count: 2,
            max: 6,
            reroll: Some(Reroll(2)),
            ..Default::default()
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn bounded_expression_with_explode() {
        let actual = parse("2d6!");
        let expected = Expression {
            count: 2,
            max: 6,
            explode: Some(Explode(6)),
            ..Default::default()
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn bounded_expression_with_explode_5() {
        let actual = parse("2d6!5");
        let expected = Expression {
            count: 2,
            max: 6,
            explode: Some(Explode(5)),
            ..Default::default()
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn bounded_expression_with_reroll_and_explode() {
        let actual = parse("2d6r!");
        let expected = Expression {
            count: 2,
            max: 6,
            reroll: Some(Reroll(1)),
            explode: Some(Explode(6)),
            ..Default::default()
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn bounded_expression_with_reroll_and_explode_non_default_thresholds() {
        let a = parse("2d6r2!5");
        let b = parse("2d6!5r2");
        let expected = Expression {
            count: 2,
            max: 6,
            reroll: Some(Reroll(2)),
            explode: Some(Explode(5)),
            ..Default::default()
        };

        assert_eq!(a, expected);
        assert_eq!(b, expected);
    }

    #[test]
    fn bounded_expression_with_advantage() {
        let a = parse("a20");
        let b = parse("a1d20");
        let c = parse("ad20");

        let expected = Expression {
            count: 1,
            max: 20,
            advantage: Advantage::Advantage,
            ..Default::default()
        };

        assert_eq!(a, expected);
        assert_eq!(b, expected);
        assert_eq!(c, expected);
    }

    #[test]
    fn bounded_expression_with_disadvantage() {
        let a = parse("s20");
        let b = parse("s1d20");

        let expected = Expression {
            count: 1,
            max: 20,
            advantage: Advantage::Disadvantage,
            ..Default::default()
        };

        assert_eq!(a, expected);
        assert_eq!(b, expected);
    }

    #[test]
    fn realize_bounded_expression() {
        let mut realizer = MockRealizer::new(vec![2, 3]);
        let expression = parse("2d6");
        assert_eq!(5, realizer.realize(&expression).sum());
    }

    #[test]
    fn realize_advantage() {
        let mut realizer = MockRealizer::new(vec![2, 20]);
        let expression = parse("a20");
        assert_eq!(20, realizer.realize(&expression).sum());
    }

    #[test]
    fn realize_disadvantage() {
        let mut realizer = MockRealizer::new(vec![20, 2]);
        let expression = parse("s20");
        assert_eq!(2, realizer.realize(&expression).sum());
    }

    #[test]
    fn realize_reroll() {
        let mut realizer = MockRealizer::new(vec![2, 3, 5]);
        let expression = parse("2d6r2");
        assert_eq!(8, realizer.realize(&expression).sum());
    }

    #[test]
    fn realize_explode() {
        let mut realizer = MockRealizer::new(vec![3, 5, 2]);
        let expression = parse("2d6!5");
        assert_eq!(10, realizer.realize(&expression).sum());
    }

    #[test]
    fn realize_reroll_and_explode() {
        let mut realizer = MockRealizer::new(vec![1, 2, 5, 6, 3, 4]);
        let expression = parse("2d6r2!5");
        assert_eq!(18, realizer.realize(&expression).sum());
    }

    // I honestly don't know what the desired result for these two tests is.
    // Let these serve to exemplify the behavior of the library rather than to
    // define correct behavior.
    // - realize_advantage_reroll_and_explode
    // - realize_disadvantage_reroll_and_explode

    #[test]
    fn realize_advantage_reroll_and_explode() {
        let mut realizer = MockRealizer::new(vec![1, 5, 3, 2]);
        let expression = parse("a2d6r!5");
        assert_eq!(10, realizer.realize(&expression).sum());
    }

    #[test]
    fn realize_disadvantage_reroll_and_explode() {
        let mut realizer = MockRealizer::new(vec![1, 5, 3, 2]);
        let expression = parse("s2d6r!5");
        assert_eq!(5, realizer.realize(&expression).sum());
    }

    fn parse(s: &str) -> Expression {
        ExpressionParser::new().parse(s).unwrap()
    }

    fn count_max(count: i32, max: i32) -> Expression {
        Expression {
            count,
            max,
            ..Default::default()
        }
    }

    struct MockRealizer<T> {
        source: T,
    }

    impl<T> MockRealizer<T> {
        fn new(source: impl IntoIterator<IntoIter = T>) -> Self {
            Self {
                source: source.into_iter(),
            }
        }
    }

    impl<T: Iterator<Item = i32>> Realizer for MockRealizer<T> {
        fn next(&mut self, _max: i32) -> i32 {
            self.source.next().unwrap()
        }
    }
}
