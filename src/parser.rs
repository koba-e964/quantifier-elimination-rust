use crate::formula::{Formula, Quantifier, Relation};
use crate::polynomial::{Polynomial, VariableNames};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    pub position: usize,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedFormula {
    pub formula: Formula,
    pub names: VariableNames,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "at byte {}: {}", self.position, self.message)
    }
}

impl std::error::Error for ParseError {}

pub fn parse_formula(input: &str) -> Result<Formula, ParseError> {
    let mut parser = Parser::new(input)?;
    let formula = parser.parse_formula()?;
    if let Some(token) = parser.peek() {
        return Err(parser.error(token.position(), "expected end of input"));
    }
    Ok(formula)
}

pub fn parse_formula_with_names(input: &str) -> Result<ParsedFormula, ParseError> {
    let mut parser = Parser::new_named(input)?;
    let formula = parser.parse_formula()?;
    if let Some(token) = parser.peek() {
        return Err(parser.error(token.position(), "expected end of input"));
    }
    Ok(ParsedFormula {
        formula,
        names: parser.names,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TokenKind {
    Identifier(String),
    Integer(i64),
    Plus,
    Minus,
    Star,
    Caret,
    LParen,
    RParen,
    Dot,
    And,
    Or,
    Not,
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Token {
    kind: TokenKind,
    position: usize,
}

impl Token {
    fn position(&self) -> usize {
        self.position
    }
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
    named: bool,
    next_variable: usize,
    free_variables: BTreeMap<String, usize>,
    scopes: Vec<BTreeMap<String, usize>>,
    names: VariableNames,
}

impl Parser {
    fn new(input: &str) -> Result<Self, ParseError> {
        Self::new_with_mode(input, false)
    }

    fn new_named(input: &str) -> Result<Self, ParseError> {
        Self::new_with_mode(input, true)
    }

    fn new_with_mode(input: &str, named: bool) -> Result<Self, ParseError> {
        Ok(Self {
            tokens: tokenize(input)?,
            index: 0,
            named,
            next_variable: 0,
            free_variables: BTreeMap::new(),
            scopes: Vec::new(),
            names: VariableNames::new(),
        })
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.index).cloned();
        self.index += usize::from(token.is_some());
        token
    }

    fn error(&self, position: usize, message: impl Into<String>) -> ParseError {
        ParseError {
            position,
            message: message.into(),
        }
    }

    fn parse_formula(&mut self) -> Result<Formula, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Formula, ParseError> {
        let first = self.parse_and()?;
        let mut formulas = vec![first];
        while self.consume(|kind| matches!(kind, TokenKind::Or)) {
            formulas.push(self.parse_and()?);
        }
        Ok(if formulas.len() == 1 {
            formulas.pop().unwrap()
        } else {
            Formula::Or(formulas)
        })
    }

    fn parse_and(&mut self) -> Result<Formula, ParseError> {
        let first = self.parse_unary_formula()?;
        let mut formulas = vec![first];
        while self.consume(|kind| matches!(kind, TokenKind::And)) {
            formulas.push(self.parse_unary_formula()?);
        }
        Ok(if formulas.len() == 1 {
            formulas.pop().unwrap()
        } else {
            Formula::And(formulas)
        })
    }

    fn parse_unary_formula(&mut self) -> Result<Formula, ParseError> {
        if self.consume(|kind| matches!(kind, TokenKind::Not)) {
            return Ok(Formula::Not(Box::new(self.parse_unary_formula()?)));
        }
        if let Some(quantifier) = self.consume_quantifier() {
            return self.parse_quantified(quantifier);
        }
        if self.consume(|kind| matches!(kind, TokenKind::LParen)) {
            let formula = self.parse_formula()?;
            self.expect(
                |kind| matches!(kind, TokenKind::RParen),
                "expected ')' after formula",
            )?;
            return Ok(formula);
        }
        if self.consume_identifier("true") {
            return Ok(Formula::True);
        }
        if self.consume_identifier("false") {
            return Ok(Formula::False);
        }
        let left = self.parse_polynomial()?;
        let relation = self.parse_relation()?;
        let right = self.parse_polynomial()?;
        Ok(Formula::atom(left - right, relation))
    }

    fn parse_quantified(&mut self, quantifier: Quantifier) -> Result<Formula, ParseError> {
        let token = self
            .next()
            .ok_or_else(|| self.error(self.end_position(), "expected variable"))?;
        let variable = match token.kind {
            TokenKind::Identifier(name) if self.named => {
                let variable = self.next_variable;
                self.next_variable += 1;
                self.scopes
                    .push([(name.clone(), variable)].into_iter().collect());
                self.names.insert(variable, name);
                variable
            }
            TokenKind::Identifier(name) => parse_variable(&name, token.position, self)?,
            _ => return Err(self.error(token.position, "expected a variable such as x0")),
        };
        self.expect(
            |kind| matches!(kind, TokenKind::Dot),
            "expected '.' after quantified variable",
        )?;
        let body = self.parse_formula()?;
        if self.named {
            self.scopes.pop();
        }
        Ok(match quantifier {
            Quantifier::Exists => Formula::exists(variable, body),
            Quantifier::Forall => Formula::forall(variable, body),
        })
    }

    fn parse_polynomial(&mut self) -> Result<Polynomial, ParseError> {
        self.parse_add_sub()
    }

    fn parse_add_sub(&mut self) -> Result<Polynomial, ParseError> {
        let mut result = self.parse_mul()?;
        loop {
            if self.consume(|kind| matches!(kind, TokenKind::Plus)) {
                result = result + self.parse_mul()?;
            } else if self.consume(|kind| matches!(kind, TokenKind::Minus)) {
                result = result - self.parse_mul()?;
            } else {
                return Ok(result);
            }
        }
    }

    fn parse_mul(&mut self) -> Result<Polynomial, ParseError> {
        let mut result = self.parse_power()?;
        while self.consume(|kind| matches!(kind, TokenKind::Star)) {
            result = result * self.parse_power()?;
        }
        Ok(result)
    }

    fn parse_power(&mut self) -> Result<Polynomial, ParseError> {
        let base = self.parse_polynomial_atom()?;
        if !self.consume(|kind| matches!(kind, TokenKind::Caret)) {
            return Ok(base);
        }
        let token = self
            .next()
            .ok_or_else(|| self.error(self.end_position(), "expected exponent"))?;
        match token.kind {
            TokenKind::Integer(exponent) if exponent >= 0 => Ok(base.pow(exponent as usize)),
            _ => Err(self.error(token.position, "expected a non-negative integer exponent")),
        }
    }

    fn parse_polynomial_atom(&mut self) -> Result<Polynomial, ParseError> {
        if self.consume(|kind| matches!(kind, TokenKind::Minus)) {
            return Ok(-self.parse_polynomial_atom()?);
        }
        if self.consume(|kind| matches!(kind, TokenKind::Plus)) {
            return self.parse_polynomial_atom();
        }
        let token = self
            .next()
            .ok_or_else(|| self.error(self.end_position(), "expected polynomial term"))?;
        match token.kind {
            TokenKind::Integer(value) => Ok(Polynomial::integer(value)),
            TokenKind::Identifier(name) if self.named => {
                Ok(Polynomial::variable(self.resolve_named_variable(name)))
            }
            TokenKind::Identifier(name) => Ok(Polynomial::variable(parse_variable(
                &name,
                token.position,
                self,
            )?)),
            TokenKind::LParen => {
                let polynomial = self.parse_polynomial()?;
                self.expect(
                    |kind| matches!(kind, TokenKind::RParen),
                    "expected ')' after polynomial",
                )?;
                Ok(polynomial)
            }
            _ => Err(self.error(
                token.position,
                "expected integer, variable, or parenthesized polynomial",
            )),
        }
    }

    fn resolve_named_variable(&mut self, name: String) -> usize {
        if let Some(variable) = self
            .scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(&name).copied())
        {
            return variable;
        }
        if let Some(variable) = self.free_variables.get(&name).copied() {
            return variable;
        }
        let variable = self.next_variable;
        self.next_variable += 1;
        self.free_variables.insert(name.clone(), variable);
        self.names.insert(variable, name);
        variable
    }

    fn parse_relation(&mut self) -> Result<Relation, ParseError> {
        let token = self
            .next()
            .ok_or_else(|| self.error(self.end_position(), "expected relation"))?;
        match token.kind {
            TokenKind::Equal => Ok(Relation::Equal),
            TokenKind::NotEqual => Ok(Relation::NotEqual),
            TokenKind::Less => Ok(Relation::Less),
            TokenKind::LessOrEqual => Ok(Relation::LessOrEqual),
            TokenKind::Greater => Ok(Relation::Greater),
            TokenKind::GreaterOrEqual => Ok(Relation::GreaterOrEqual),
            _ => Err(self.error(token.position, "expected a comparison relation")),
        }
    }

    fn consume_quantifier(&mut self) -> Option<Quantifier> {
        match self.peek().map(|token| &token.kind) {
            Some(TokenKind::Identifier(name)) if name == "exists" => {
                self.next();
                Some(Quantifier::Exists)
            }
            Some(TokenKind::Identifier(name)) if name == "forall" => {
                self.next();
                Some(Quantifier::Forall)
            }
            _ => None,
        }
    }

    fn consume_identifier(&mut self, expected: &str) -> bool {
        matches!(self.peek().map(|token| &token.kind), Some(TokenKind::Identifier(name)) if name == expected)
            && self.next().is_some()
    }

    fn consume(&mut self, predicate: impl FnOnce(&TokenKind) -> bool) -> bool {
        self.peek().is_some_and(|token| predicate(&token.kind)) && self.next().is_some()
    }

    fn expect(
        &mut self,
        predicate: impl FnOnce(&TokenKind) -> bool,
        message: &str,
    ) -> Result<(), ParseError> {
        match self.next() {
            Some(token) if predicate(&token.kind) => Ok(()),
            Some(token) => Err(self.error(token.position, message)),
            None => Err(self.error(self.end_position(), message)),
        }
    }

    fn end_position(&self) -> usize {
        self.tokens.last().map_or(0, |token| token.position + 1)
    }
}

fn parse_variable(name: &str, position: usize, parser: &Parser) -> Result<usize, ParseError> {
    let Some(number) = name.strip_prefix('x') else {
        return Err(parser.error(position, "variables must use the form x0, x1, ..."));
    };
    if number.is_empty() {
        return Err(parser.error(position, "variables must use the form x0, x1, ..."));
    }
    number
        .parse()
        .map_err(|_| parser.error(position, "variable index is too large"))
}

fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let bytes = input.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }
        let position = index;
        let token = match bytes[index] {
            b'0'..=b'9' => {
                index += 1;
                while index < bytes.len() && bytes[index].is_ascii_digit() {
                    index += 1;
                }
                let value = input[position..index].parse().map_err(|_| ParseError {
                    position,
                    message: "integer literal is too large".to_owned(),
                })?;
                TokenKind::Integer(value)
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                index += 1;
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
                {
                    index += 1;
                }
                TokenKind::Identifier(input[position..index].to_owned())
            }
            b'+' => {
                index += 1;
                TokenKind::Plus
            }
            b'-' => {
                index += 1;
                TokenKind::Minus
            }
            b'*' => {
                index += 1;
                TokenKind::Star
            }
            b'^' => {
                index += 1;
                TokenKind::Caret
            }
            b'(' => {
                index += 1;
                TokenKind::LParen
            }
            b')' => {
                index += 1;
                TokenKind::RParen
            }
            b'.' => {
                index += 1;
                TokenKind::Dot
            }
            b'!' if bytes.get(index + 1) == Some(&b'=') => {
                index += 2;
                TokenKind::NotEqual
            }
            b'!' => {
                index += 1;
                TokenKind::Not
            }
            b'&' if bytes.get(index + 1) == Some(&b'&') => {
                index += 2;
                TokenKind::And
            }
            b'|' if bytes.get(index + 1) == Some(&b'|') => {
                index += 2;
                TokenKind::Or
            }
            b'=' => {
                index += 1;
                TokenKind::Equal
            }
            b'<' if bytes.get(index + 1) == Some(&b'=') => {
                index += 2;
                TokenKind::LessOrEqual
            }
            b'<' => {
                index += 1;
                TokenKind::Less
            }
            b'>' if bytes.get(index + 1) == Some(&b'=') => {
                index += 2;
                TokenKind::GreaterOrEqual
            }
            b'>' => {
                index += 1;
                TokenKind::Greater
            }
            _ => {
                return Err(ParseError {
                    position,
                    message: format!(
                        "unexpected character {:?}",
                        input[position..].chars().next().unwrap()
                    ),
                });
            }
        };
        tokens.push(Token {
            kind: token,
            position,
        });
    }
    Ok(tokens)
}
