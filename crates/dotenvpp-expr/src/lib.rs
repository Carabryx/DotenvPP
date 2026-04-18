//! Safe expression parser and evaluator for DotenvPP.
//!
//! Expressions intentionally have no loops, no user-defined functions, and no
//! implicit I/O. File and process-env access are opt-in through [`EvalOptions`].

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Expression evaluation errors.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ExprError {
    /// The lexer found invalid syntax.
    #[error("lex error at byte {position}: {message}")]
    Lex {
        /// Byte position.
        position: usize,
        /// Error message.
        message: String,
    },
    /// The parser found invalid syntax.
    #[error("parse error: {0}")]
    Parse(String),
    /// Evaluation failed.
    #[error("evaluation error: {0}")]
    Eval(String),
    /// I/O was requested but sandbox policy denied it.
    #[error("I/O is disabled in this expression sandbox")]
    IoDisabled,
    /// OS environment access was requested but sandbox policy denied it.
    #[error("OS environment access is disabled in this expression sandbox")]
    EnvDisabled,
}

/// Runtime value produced by expression evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// String value.
    String(String),
    /// Number value.
    Number(f64),
    /// Boolean value.
    Bool(bool),
    /// Null value.
    Null,
}

impl Value {
    /// Render this value for `.env` output.
    pub fn to_env_string(&self) -> String {
        match self {
            Self::String(value) => value.clone(),
            Self::Number(value) => {
                if value.fract() == 0.0 {
                    format!("{value:.0}")
                } else {
                    value.to_string()
                }
            }
            Self::Bool(value) => value.to_string(),
            Self::Null => String::new(),
        }
    }

    fn as_bool(&self) -> bool {
        match self {
            Self::Bool(value) => *value,
            Self::Number(value) => *value != 0.0,
            Self::String(value) => {
                !value.is_empty() && !matches!(value.to_ascii_lowercase().as_str(), "false" | "0")
            }
            Self::Null => false,
        }
    }

    fn as_number(&self) -> Result<f64, ExprError> {
        match self {
            Self::Number(value) => Ok(*value),
            Self::Bool(value) => Ok(u8::from(*value).into()),
            Self::String(value) => value
                .parse::<f64>()
                .map_err(|_| ExprError::Eval(format!("`{value}` is not numeric"))),
            Self::Null => Err(ExprError::Eval("null is not numeric".to_owned())),
        }
    }
}

/// Evaluation options and sandbox knobs.
#[derive(Debug, Clone)]
pub struct EvalOptions {
    /// Variables visible to the expression.
    pub variables: HashMap<String, String>,
    /// Allow `env("NAME")` to read process environment variables.
    pub allow_env: bool,
    /// Allow `file("path")` to read from disk.
    pub allow_io: bool,
    /// Root directory for file reads. Paths must stay under this root.
    pub file_root: PathBuf,
    /// Maximum expression length in bytes.
    pub max_len: usize,
    /// Maximum AST recursion depth during evaluation.
    pub max_depth: usize,
}

impl Default for EvalOptions {
    fn default() -> Self {
        Self {
            variables: HashMap::new(),
            allow_env: false,
            allow_io: false,
            file_root: PathBuf::from("."),
            max_len: 16 * 1024,
            max_depth: 64,
        }
    }
}

/// Full evaluation result with determinism metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct EvalOutput {
    /// Final value.
    pub value: Value,
    /// False if evaluation used `uuid()`, `now()`, `env()`, or `file()`.
    pub deterministic: bool,
}

/// Evaluate an expression with default sandbox options and supplied variables.
pub fn eval(expression: &str, variables: &HashMap<String, String>) -> Result<Value, ExprError> {
    let options = EvalOptions {
        variables: variables.clone(),
        ..Default::default()
    };
    evaluate(expression, &options).map(|output| output.value)
}

/// Evaluate an expression using explicit options.
pub fn evaluate(expression: &str, options: &EvalOptions) -> Result<EvalOutput, ExprError> {
    if expression.len() > options.max_len {
        return Err(ExprError::Parse(format!(
            "expression is too large: {} bytes > {} bytes",
            expression.len(),
            options.max_len
        )));
    }

    let tokens = Lexer::new(expression).lex()?;
    let ast = Parser::new(tokens).parse()?;
    let mut runtime = Runtime {
        options,
        deterministic: true,
    };
    let value = runtime.eval(&ast, 0)?;
    Ok(EvalOutput {
        value,
        deterministic: runtime.deterministic,
    })
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    String(String),
    Ident(String),
    Var(String),
    True,
    False,
    Null,
    If,
    Then,
    Else,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    EqEq,
    NotEq,
    Lt,
    Lte,
    Gt,
    Gte,
    AndAnd,
    OrOr,
    Bang,
    Implies,
    LParen,
    RParen,
    Comma,
    Eof,
}

struct Lexer<'a> {
    input: &'a str,
    cursor: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            cursor: 0,
        }
    }

    fn lex(mut self) -> Result<Vec<Token>, ExprError> {
        let mut tokens = Vec::new();

        while self.cursor < self.input.len() {
            let ch = self.peek_char().unwrap_or('\0');
            match ch {
                ' ' | '\t' | '\r' | '\n' => {
                    self.bump_char();
                }
                '0'..='9' => tokens.push(self.number()?),
                '"' | '\'' => tokens.push(Token::String(self.string(ch)?)),
                '$' => tokens.push(Token::Var(self.variable()?)),
                '+' => {
                    self.bump_char();
                    tokens.push(Token::Plus);
                }
                '-' => {
                    self.bump_char();
                    tokens.push(Token::Minus);
                }
                '*' => {
                    self.bump_char();
                    tokens.push(Token::Star);
                }
                '/' => {
                    self.bump_char();
                    tokens.push(Token::Slash);
                }
                '%' => {
                    self.bump_char();
                    tokens.push(Token::Percent);
                }
                '(' => {
                    self.bump_char();
                    tokens.push(Token::LParen);
                }
                ')' => {
                    self.bump_char();
                    tokens.push(Token::RParen);
                }
                ',' => {
                    self.bump_char();
                    tokens.push(Token::Comma);
                }
                '!' => {
                    self.bump_char();
                    if self.consume('=') {
                        tokens.push(Token::NotEq);
                    } else {
                        tokens.push(Token::Bang);
                    }
                }
                '=' => {
                    self.bump_char();
                    if self.consume('=') {
                        tokens.push(Token::EqEq);
                    } else if self.consume('>') {
                        tokens.push(Token::Implies);
                    } else {
                        return Err(self.error("expected `==` or `=>`"));
                    }
                }
                '<' => {
                    self.bump_char();
                    if self.consume('=') {
                        tokens.push(Token::Lte);
                    } else {
                        tokens.push(Token::Lt);
                    }
                }
                '>' => {
                    self.bump_char();
                    if self.consume('=') {
                        tokens.push(Token::Gte);
                    } else {
                        tokens.push(Token::Gt);
                    }
                }
                '&' => {
                    self.bump_char();
                    if self.consume('&') {
                        tokens.push(Token::AndAnd);
                    } else {
                        return Err(self.error("expected `&&`"));
                    }
                }
                '|' => {
                    self.bump_char();
                    if self.consume('|') {
                        tokens.push(Token::OrOr);
                    } else {
                        return Err(self.error("expected `||`"));
                    }
                }
                '_' | 'a'..='z' | 'A'..='Z' => tokens.push(self.ident()),
                _ => return Err(self.error(format!("unexpected character `{ch}`"))),
            }
        }

        tokens.push(Token::Eof);
        Ok(tokens)
    }

    fn number(&mut self) -> Result<Token, ExprError> {
        let start = self.cursor;
        while matches!(self.peek_char(), Some('0'..='9')) {
            self.bump_char();
        }
        if self.peek_char() == Some('.') {
            self.bump_char();
            while matches!(self.peek_char(), Some('0'..='9')) {
                self.bump_char();
            }
        }
        let value = self.input[start..self.cursor].parse::<f64>().map_err(|_| ExprError::Lex {
            position: start,
            message: "invalid number".to_owned(),
        })?;
        Ok(Token::Number(value))
    }

    fn string(&mut self, quote: char) -> Result<String, ExprError> {
        let start = self.cursor;
        self.bump_char();
        let mut value = String::new();

        while let Some(ch) = self.bump_char() {
            if ch == quote {
                return Ok(value);
            }
            if ch == '\\' {
                let escaped = self.bump_char().ok_or_else(|| ExprError::Lex {
                    position: self.cursor,
                    message: "unterminated escape sequence".to_owned(),
                })?;
                match escaped {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    '\'' => value.push('\''),
                    other => {
                        value.push('\\');
                        value.push(other);
                    }
                }
            } else {
                value.push(ch);
            }
        }

        Err(ExprError::Lex {
            position: start,
            message: "unterminated string".to_owned(),
        })
    }

    fn variable(&mut self) -> Result<String, ExprError> {
        let start = self.cursor;
        self.bump_char();
        if self.consume('{') {
            let name_start = self.cursor;
            while let Some(ch) = self.peek_char() {
                if ch == '}' {
                    let name = &self.input[name_start..self.cursor];
                    self.bump_char();
                    if is_valid_var_name(name) {
                        return Ok(name.to_owned());
                    }
                    return Err(ExprError::Lex {
                        position: name_start,
                        message: "invalid variable name".to_owned(),
                    });
                }
                self.bump_char();
            }
            return Err(ExprError::Lex {
                position: start,
                message: "unterminated `${...}` variable".to_owned(),
            });
        }

        let name_start = self.cursor;
        while matches!(self.peek_char(), Some('_' | '.' | 'a'..='z' | 'A'..='Z' | '0'..='9')) {
            self.bump_char();
        }
        let name = &self.input[name_start..self.cursor];
        if is_valid_var_name(name) {
            Ok(name.to_owned())
        } else {
            Err(ExprError::Lex {
                position: start,
                message: "invalid `$VAR` reference".to_owned(),
            })
        }
    }

    fn ident(&mut self) -> Token {
        let start = self.cursor;
        while matches!(self.peek_char(), Some('_' | '.' | '-' | 'a'..='z' | 'A'..='Z' | '0'..='9'))
        {
            self.bump_char();
        }
        match &self.input[start..self.cursor] {
            "if" => Token::If,
            "then" => Token::Then,
            "else" => Token::Else,
            "true" => Token::True,
            "false" => Token::False,
            "null" => Token::Null,
            ident => Token::Ident(ident.to_owned()),
        }
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.peek_char() == Some(expected) {
            self.bump_char();
            true
        } else {
            false
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.cursor..].chars().next()
    }

    fn bump_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.cursor += ch.len_utf8();
        Some(ch)
    }

    fn error(&self, message: impl Into<String>) -> ExprError {
        ExprError::Lex {
            position: self.cursor,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Value(Value),
    Var(String),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
    And,
    Or,
    Implies,
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            cursor: 0,
        }
    }

    fn parse(mut self) -> Result<Expr, ExprError> {
        let expr = self.parse_implication()?;
        if !matches!(self.peek(), Token::Eof) {
            return Err(ExprError::Parse(format!("unexpected token {:?}", self.peek())));
        }
        Ok(expr)
    }

    fn parse_implication(&mut self) -> Result<Expr, ExprError> {
        let mut expr = self.parse_or()?;
        while matches!(self.peek(), Token::Implies) {
            self.bump();
            let right = self.parse_implication()?;
            expr = Expr::Binary {
                op: BinaryOp::Implies,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr, ExprError> {
        let mut expr = self.parse_and()?;
        while matches!(self.peek(), Token::OrOr) {
            self.bump();
            let right = self.parse_and()?;
            expr = Expr::Binary {
                op: BinaryOp::Or,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, ExprError> {
        let mut expr = self.parse_comparison()?;
        while matches!(self.peek(), Token::AndAnd) {
            self.bump();
            let right = self.parse_comparison()?;
            expr = Expr::Binary {
                op: BinaryOp::And,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ExprError> {
        let mut expr = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Token::EqEq => BinaryOp::Eq,
                Token::NotEq => BinaryOp::Ne,
                Token::Lt => BinaryOp::Lt,
                Token::Lte => BinaryOp::Lte,
                Token::Gt => BinaryOp::Gt,
                Token::Gte => BinaryOp::Gte,
                _ => break,
            };
            self.bump();
            let right = self.parse_additive()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_additive(&mut self) -> Result<Expr, ExprError> {
        let mut expr = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.bump();
            let right = self.parse_multiplicative()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ExprError> {
        let mut expr = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                Token::Percent => BinaryOp::Rem,
                _ => break,
            };
            self.bump();
            let right = self.parse_unary()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, ExprError> {
        match self.peek() {
            Token::Minus => {
                self.bump();
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            Token::Bang => {
                self.bump();
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, ExprError> {
        match self.bump() {
            Token::Number(value) => Ok(Expr::Value(Value::Number(value))),
            Token::String(value) => Ok(Expr::Value(Value::String(value))),
            Token::True => Ok(Expr::Value(Value::Bool(true))),
            Token::False => Ok(Expr::Value(Value::Bool(false))),
            Token::Null => Ok(Expr::Value(Value::Null)),
            Token::Var(name) => Ok(Expr::Var(name)),
            Token::Ident(name) if matches!(self.peek(), Token::LParen) => {
                self.bump();
                let mut args = Vec::new();
                if !matches!(self.peek(), Token::RParen) {
                    loop {
                        args.push(self.parse_implication()?);
                        if matches!(self.peek(), Token::Comma) {
                            self.bump();
                            continue;
                        }
                        break;
                    }
                }
                self.expect(Token::RParen)?;
                Ok(Expr::Call {
                    name,
                    args,
                })
            }
            Token::Ident(name) => Ok(Expr::Var(name)),
            Token::If => {
                let condition = self.parse_implication()?;
                self.expect(Token::Then)?;
                let then_branch = self.parse_implication()?;
                self.expect(Token::Else)?;
                let else_branch = self.parse_implication()?;
                Ok(Expr::If {
                    condition: Box::new(condition),
                    then_branch: Box::new(then_branch),
                    else_branch: Box::new(else_branch),
                })
            }
            Token::LParen => {
                let expr = self.parse_implication()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            other => Err(ExprError::Parse(format!("unexpected token {other:?}"))),
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), ExprError> {
        let actual = self.bump();
        if std::mem::discriminant(&actual) == std::mem::discriminant(&expected) {
            Ok(())
        } else {
            Err(ExprError::Parse(format!("expected {expected:?}, found {actual:?}")))
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn bump(&mut self) -> Token {
        let token = self.tokens[self.cursor].clone();
        self.cursor += 1;
        token
    }
}

struct Runtime<'a> {
    options: &'a EvalOptions,
    deterministic: bool,
}

impl Runtime<'_> {
    fn eval(&mut self, expr: &Expr, depth: usize) -> Result<Value, ExprError> {
        if depth > self.options.max_depth {
            return Err(ExprError::Eval("expression recursion depth exceeded".to_owned()));
        }

        match expr {
            Expr::Value(value) => Ok(value.clone()),
            Expr::Var(name) => Ok(self
                .options
                .variables
                .get(name)
                .cloned()
                .map(Value::String)
                .unwrap_or(Value::Null)),
            Expr::Unary {
                op,
                expr,
            } => {
                let value = self.eval(expr, depth + 1)?;
                match op {
                    UnaryOp::Neg => Ok(Value::Number(-value.as_number()?)),
                    UnaryOp::Not => Ok(Value::Bool(!value.as_bool())),
                }
            }
            Expr::Binary {
                op,
                left,
                right,
            } => self.eval_binary(*op, left, right, depth),
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                if self.eval(condition, depth + 1)?.as_bool() {
                    self.eval(then_branch, depth + 1)
                } else {
                    self.eval(else_branch, depth + 1)
                }
            }
            Expr::Call {
                name,
                args,
            } => self.call(name, args, depth),
        }
    }

    fn eval_binary(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        depth: usize,
    ) -> Result<Value, ExprError> {
        match op {
            BinaryOp::And => {
                let lhs = self.eval(left, depth + 1)?;
                if !lhs.as_bool() {
                    return Ok(Value::Bool(false));
                }
                Ok(Value::Bool(self.eval(right, depth + 1)?.as_bool()))
            }
            BinaryOp::Or => {
                let lhs = self.eval(left, depth + 1)?;
                if lhs.as_bool() {
                    return Ok(Value::Bool(true));
                }
                Ok(Value::Bool(self.eval(right, depth + 1)?.as_bool()))
            }
            BinaryOp::Implies => {
                let lhs = self.eval(left, depth + 1)?;
                if !lhs.as_bool() {
                    return Ok(Value::Bool(true));
                }
                Ok(Value::Bool(self.eval(right, depth + 1)?.as_bool()))
            }
            _ => {
                let lhs = self.eval(left, depth + 1)?;
                let rhs = self.eval(right, depth + 1)?;
                match op {
                    BinaryOp::Add => {
                        if matches!(lhs, Value::String(_)) || matches!(rhs, Value::String(_)) {
                            Ok(Value::String(format!(
                                "{}{}",
                                lhs.to_env_string(),
                                rhs.to_env_string()
                            )))
                        } else {
                            Ok(Value::Number(lhs.as_number()? + rhs.as_number()?))
                        }
                    }
                    BinaryOp::Sub => Ok(Value::Number(lhs.as_number()? - rhs.as_number()?)),
                    BinaryOp::Mul => Ok(Value::Number(lhs.as_number()? * rhs.as_number()?)),
                    BinaryOp::Div => {
                        let divisor = rhs.as_number()?;
                        if divisor == 0.0 {
                            return Err(ExprError::Eval("division by zero".to_owned()));
                        }
                        Ok(Value::Number(lhs.as_number()? / divisor))
                    }
                    BinaryOp::Rem => {
                        let divisor = rhs.as_number()?;
                        if divisor == 0.0 {
                            return Err(ExprError::Eval("modulo by zero".to_owned()));
                        }
                        Ok(Value::Number(lhs.as_number()? % divisor))
                    }
                    BinaryOp::Eq => Ok(Value::Bool(values_equal(&lhs, &rhs))),
                    BinaryOp::Ne => Ok(Value::Bool(!values_equal(&lhs, &rhs))),
                    BinaryOp::Lt => Ok(Value::Bool(compare_values(&lhs, &rhs)? < 0)),
                    BinaryOp::Lte => Ok(Value::Bool(compare_values(&lhs, &rhs)? <= 0)),
                    BinaryOp::Gt => Ok(Value::Bool(compare_values(&lhs, &rhs)? > 0)),
                    BinaryOp::Gte => Ok(Value::Bool(compare_values(&lhs, &rhs)? >= 0)),
                    BinaryOp::And | BinaryOp::Or | BinaryOp::Implies => unreachable!(),
                }
            }
        }
    }

    fn call(&mut self, name: &str, args: &[Expr], depth: usize) -> Result<Value, ExprError> {
        let values = args
            .iter()
            .map(|arg| self.eval(arg, depth + 1))
            .collect::<Result<Vec<_>, _>>()?;

        match name {
            "len" => {
                expect_arity(name, &values, 1)?;
                Ok(Value::Number(values[0].to_env_string().chars().count() as f64))
            }
            "upper" => {
                expect_arity(name, &values, 1)?;
                Ok(Value::String(values[0].to_env_string().to_ascii_uppercase()))
            }
            "lower" => {
                expect_arity(name, &values, 1)?;
                Ok(Value::String(values[0].to_env_string().to_ascii_lowercase()))
            }
            "trim" => {
                expect_arity(name, &values, 1)?;
                Ok(Value::String(values[0].to_env_string().trim().to_owned()))
            }
            "contains" => {
                expect_arity(name, &values, 2)?;
                Ok(Value::Bool(
                    values[0].to_env_string().contains(values[1].to_env_string().as_str()),
                ))
            }
            "starts_with" => {
                expect_arity(name, &values, 2)?;
                Ok(Value::Bool(
                    values[0].to_env_string().starts_with(values[1].to_env_string().as_str()),
                ))
            }
            "ends_with" => {
                expect_arity(name, &values, 2)?;
                Ok(Value::Bool(
                    values[0].to_env_string().ends_with(values[1].to_env_string().as_str()),
                ))
            }
            "concat" => Ok(Value::String(
                values.iter().map(Value::to_env_string).collect::<Vec<_>>().join(""),
            )),
            "sha256" => {
                expect_arity(name, &values, 1)?;
                let mut hasher = Sha256::new();
                hasher.update(values[0].to_env_string().as_bytes());
                Ok(Value::String(format!("{:x}", hasher.finalize())))
            }
            "base64_encode" => {
                expect_arity(name, &values, 1)?;
                Ok(Value::String(BASE64.encode(values[0].to_env_string())))
            }
            "base64_decode" => {
                expect_arity(name, &values, 1)?;
                let decoded = BASE64
                    .decode(values[0].to_env_string())
                    .map_err(|err| ExprError::Eval(format!("base64 decode failed: {err}")))?;
                let decoded = String::from_utf8(decoded).map_err(|err| {
                    ExprError::Eval(format!("decoded bytes are not UTF-8: {err}"))
                })?;
                Ok(Value::String(decoded))
            }
            "duration" => {
                expect_arity(name, &values, 1)?;
                Ok(Value::Number(parse_duration_seconds(&values[0].to_env_string())? as f64))
            }
            "uuid" => {
                expect_arity(name, &values, 0)?;
                self.deterministic = false;
                Ok(Value::String(generate_uuid_like()))
            }
            "now" => {
                expect_arity(name, &values, 0)?;
                self.deterministic = false;
                let seconds = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|err| ExprError::Eval(format!("system clock error: {err}")))?
                    .as_secs();
                Ok(Value::String(seconds.to_string()))
            }
            "env" => {
                expect_arity(name, &values, 1)?;
                if !self.options.allow_env {
                    return Err(ExprError::EnvDisabled);
                }
                self.deterministic = false;
                Ok(Value::String(std::env::var(values[0].to_env_string()).unwrap_or_default()))
            }
            "file" => {
                expect_arity(name, &values, 1)?;
                if !self.options.allow_io {
                    return Err(ExprError::IoDisabled);
                }
                self.deterministic = false;
                let path = values[0].to_env_string();
                read_file_in_root(&self.options.file_root, Path::new(&path))
                    .map(Value::String)
                    .map_err(|err| ExprError::Eval(err.to_string()))
            }
            other => Err(ExprError::Eval(format!("unknown function `{other}`"))),
        }
    }
}

fn expect_arity(name: &str, values: &[Value], expected: usize) -> Result<(), ExprError> {
    if values.len() == expected {
        Ok(())
    } else {
        Err(ExprError::Eval(format!(
            "`{name}` expects {expected} argument(s), got {}",
            values.len()
        )))
    }
}

fn values_equal(lhs: &Value, rhs: &Value) -> bool {
    match (lhs, rhs) {
        (Value::Number(a), Value::Number(b)) => (*a - *b).abs() < f64::EPSILON,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Null, Value::Null) => true,
        _ => lhs.to_env_string() == rhs.to_env_string(),
    }
}

fn compare_values(lhs: &Value, rhs: &Value) -> Result<i8, ExprError> {
    if let (Ok(a), Ok(b)) = (lhs.as_number(), rhs.as_number()) {
        return Ok(if a < b {
            -1
        } else if a > b {
            1
        } else {
            0
        });
    }

    Ok(lhs.to_env_string().cmp(&rhs.to_env_string()) as i8)
}

fn read_file_in_root(root: &Path, relative: &Path) -> Result<String, std::io::Error> {
    let root = root.canonicalize()?;
    let joined = root.join(relative);
    let canonical = joined.canonicalize()?;
    if !canonical.starts_with(&root) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "file path escapes configured root",
        ));
    }
    std::fs::read_to_string(canonical)
}

fn generate_uuid_like() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let counter = u128::from(COUNTER.fetch_add(1, Ordering::Relaxed));
    let mixed = nanos ^ (counter << 64);

    format!(
        "{:08x}-{:04x}-4{:03x}-a{:03x}-{:012x}",
        (mixed >> 96) as u32,
        ((mixed >> 80) & 0xffff) as u16,
        ((mixed >> 64) & 0x0fff) as u16,
        ((mixed >> 48) & 0x0fff) as u16,
        mixed & 0xffff_ffff_ffff
    )
}

fn parse_duration_seconds(input: &str) -> Result<u64, ExprError> {
    let trimmed = input.trim();
    let split_at = trimmed.find(|ch: char| !ch.is_ascii_digit()).unwrap_or(trimmed.len());
    let amount = trimmed[..split_at]
        .parse::<u64>()
        .map_err(|_| ExprError::Eval("duration amount must be an unsigned integer".to_owned()))?;
    let unit = &trimmed[split_at..];
    let multiplier = match unit {
        "" | "s" | "sec" | "secs" | "second" | "seconds" => 1,
        "m" | "min" | "mins" | "minute" | "minutes" => 60,
        "h" | "hr" | "hrs" | "hour" | "hours" => 60 * 60,
        "d" | "day" | "days" => 24 * 60 * 60,
        "ms" => return Ok(amount / 1000),
        other => return Err(ExprError::Eval(format!("unsupported duration unit `{other}`"))),
    };
    amount
        .checked_mul(multiplier)
        .ok_or_else(|| ExprError::Eval("duration overflow".to_owned()))
}

fn is_valid_var_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != b'_' {
        return false;
    }
    bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'.')
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn vars(entries: &[(&str, &str)]) -> HashMap<String, String> {
        entries
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn evaluates_arithmetic_and_variables() {
        let value = eval("${CPU_COUNT} * 2 + 1", &vars(&[("CPU_COUNT", "4")])).unwrap();
        assert_eq!(value.to_env_string(), "9");
    }

    #[test]
    fn evaluates_conditionals_and_strings() {
        let value = eval(
            r#"if ${ENV} == "production" then "warn" else "debug""#,
            &vars(&[("ENV", "production")]),
        )
        .unwrap();
        assert_eq!(value, Value::String("warn".to_owned()));
    }

    #[test]
    fn evaluates_functions() {
        let value = eval(r#"concat(upper("api"), "-", duration("2m"))"#, &HashMap::new()).unwrap();
        assert_eq!(value.to_env_string(), "API-120");

        let hash = eval(r#"sha256("abc")"#, &HashMap::new()).unwrap();
        assert_eq!(
            hash.to_env_string(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sandbox_blocks_env_and_file_by_default() {
        let err = evaluate(r#"env("PATH")"#, &EvalOptions::default()).unwrap_err();
        assert_eq!(err, ExprError::EnvDisabled);

        let err = evaluate(r#"file("Cargo.toml")"#, &EvalOptions::default()).unwrap_err();
        assert_eq!(err, ExprError::IoDisabled);
    }

    #[test]
    fn supports_policy_implication() {
        let value = eval(
            r#"ENV == "production" => contains(DATABASE_URL, "sslmode=require")"#,
            &vars(&[("ENV", "production"), ("DATABASE_URL", "postgres://db/app?sslmode=require")]),
        )
        .unwrap();
        assert_eq!(value, Value::Bool(true));
    }

    #[test]
    fn tracks_non_determinism() {
        let output = evaluate("uuid()", &EvalOptions::default()).unwrap();
        assert!(!output.deterministic);
        assert!(!output.value.to_env_string().is_empty());
    }
}
