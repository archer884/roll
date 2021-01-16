use std::{iter::FromIterator, str::FromStr};

use crate::{bounded_rng::BoundedRngProvider, error::Error, Result};

#[derive(Clone, Debug)]
pub struct Expression {
    segments: Vec<PartialExpression>,
}

impl FromIterator<PartialExpression> for Expression {
    fn from_iter<T: IntoIterator<Item = PartialExpression>>(iter: T) -> Self {
        Self {
            segments: iter.into_iter().collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExpressionResult {
    segments: Vec<SegmentResult>,
}

impl FromIterator<SegmentResult> for ExpressionResult {
    fn from_iter<T: IntoIterator<Item = SegmentResult>>(iter: T) -> Self {
        Self {
            segments: iter.into_iter().collect(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Highlight {
    Min,
    Max,
    Std,
}

#[derive(Clone, Debug)]
pub struct SegmentResult {
    value: i32,
    highlight: Highlight,
}

impl Expression {
    pub fn realize(&self, provider: &mut BoundedRngProvider) -> ExpressionResult {
        self.segments.iter().map(|x| x.realize(provider)).collect()
    }
}

#[derive(Clone, Debug)]
pub enum PartialExpression {
    Dice(DiceExpression),
    Modifier(i32),
}

impl PartialExpression {
    fn realize(&self, provider: &mut BoundedRngProvider) -> SegmentResult {
        match self {
            PartialExpression::Dice(exp) => realize_dice_expression(exp, provider),
            PartialExpression::Modifier(modifier) => SegmentResult {
                value: *modifier,
                highlight: Highlight::Std,
            },
        }
    }
}

fn realize_dice_expression(
    exp: &DiceExpression,
    provider: &mut BoundedRngProvider,
) -> SegmentResult {
    let max = exp.max;
    let min = exp.count * if exp.max.is_positive() { 1 } else { -1 };
    let sum: i32 = (0..exp.count).map(|_| provider.next(max)).sum();

    match sum {
        sum if sum == max => SegmentResult {
            highlight: Highlight::Max,
            value: sum,
        },
        sum if sum == min => SegmentResult {
            highlight: Highlight::Min,
            value: sum,
        },
        sum => SegmentResult {
            highlight: Highlight::Std,
            value: sum,
        },
    }
}

#[derive(Clone, Debug)]
pub struct DiceExpression {
    count: i32,
    max: i32,
}

impl FromStr for Expression {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        SegmentIter::new(s).map(|s| s.parse()).collect()
    }
}

impl FromStr for PartialExpression {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.bytes().any(|u| u.to_ascii_lowercase() == b'd') {
            parse_dice_expression(s).map(|exp| PartialExpression::Dice(exp))
        } else {
            let modifier = s.parse()?;
            Ok(PartialExpression::Modifier(modifier))
        }
    }
}

fn parse_dice_expression(s: &str) -> Result<DiceExpression> {
    let (is_negative, s) = if s.starts_with('-') {
        (true, &s[1..])
    } else {
        (false, s)
    };

    let mut parts = s
        .trim_start_matches(|u| u == 'd' || u == 'D')
        .split(|u| u == 'd' || u == 'D');
    let left = dbg!(parts
        .next()
        .ok_or_else(|| Error::BadExpression(s.to_string()))?);
    let right = parts.next();

    if parts.next().is_some() {
        return Err(Error::BadExpression(s.to_string()));
    }

    match right {
        Some(right) => Ok(DiceExpression {
            count: left.parse()?,
            max: if is_negative {
                -right.parse()?
            } else {
                right.parse()?
            },
        }),
        None => Ok(DiceExpression {
            count: 1,
            max: if is_negative {
                -left.parse()?
            } else {
                left.parse()?
            },
        }),
    }
}

struct SegmentIter<'a> {
    raw_expression: &'a str,
}

impl<'a> SegmentIter<'a> {
    fn new(s: &'a str) -> Self {
        Self { raw_expression: s }
    }
}

impl<'a> Iterator for SegmentIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.raw_expression.is_empty() {
            return None;
        }

        match self.raw_expression.rfind(|u| u == '+' || u == '-') {
            Some(idx) => {
                let result = &self.raw_expression[idx..];
                self.raw_expression = &self.raw_expression[..idx];
                Some(result)
            }

            None => {
                let result = self.raw_expression;
                self.raw_expression = "";
                Some(result)
            }
        }
    }
}
