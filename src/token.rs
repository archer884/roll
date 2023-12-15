use regex::Regex;

pub trait TokenExtractor {
    /// returns a boolean signifying whether or not a token was found along with an optional
    /// associated value.
    fn extract<'a>(&self, text: &'a str) -> (bool, Option<&'a str>);
}

pub struct ExplodeTokenExtractor {
    expr: Regex,
}

impl ExplodeTokenExtractor {
    fn new() -> Self {
        Self {
            expr: Regex::new(r#"(!|e)(\d+)?"#).unwrap(),
        }
    }
}

impl Default for ExplodeTokenExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenExtractor for ExplodeTokenExtractor {
    fn extract<'a>(&self, text: &'a str) -> (bool, Option<&'a str>) {
        self.expr
            .captures(text)
            .map(|cx| (true, cx.get(2).map(|cx| cx.as_str())))
            .unwrap_or((false, None))
    }
}

pub struct RerollTokenExtractor {
    expr: Regex,
}

impl RerollTokenExtractor {
    fn new() -> Self {
        Self {
            expr: Regex::new(r#"r(\d+)?"#).unwrap(),
        }
    }
}

impl Default for RerollTokenExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenExtractor for RerollTokenExtractor {
    fn extract<'a>(&self, text: &'a str) -> (bool, Option<&'a str>) {
        self.expr
            .captures(text)
            .map(|cx| (true, cx.get(1).map(|cx| cx.as_str())))
            .unwrap_or((false, None))
    }
}
