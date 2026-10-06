use super::{MAX_DEPTH, MAX_INPUT, MAX_VALUES, Result, fail};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct Value {
    pub span: Span,
    pub kind: Kind,
}

#[derive(Clone, Debug, Serialize)]
pub enum Kind {
    String(String),
    Integer(u64),
    Boolean(bool),
    Reference(Vec<String>),
    List(Vec<Value>),
    Block(Block),
    Construct(String, Block),
    Call(Vec<String>, Vec<(Option<String>, Value)>),
    Tagged(String, String),
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Block {
    pub fields: BTreeMap<String, Value>,
    pub exports: Vec<Export>,
    pub configs: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Export {
    pub kind: String,
    pub name: String,
    pub path: String,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub struct Declaration {
    pub kind: String,
    pub name: String,
    pub span: Span,
    pub parameters: BTreeMap<String, String>,
    pub block: Block,
}

#[derive(Clone, Debug, Serialize)]
pub struct Import {
    pub names: Vec<String>,
    pub path: String,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub struct Module {
    pub file: String,
    pub namespace: Option<String>,
    pub imports: Vec<Import>,
    pub declarations: BTreeMap<String, Declaration>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum TokenKind {
    Word(String),
    String(String),
    Raw(String),
    Number(u64),
    Symbol(char),
    Comment(String),
    End,
}
#[derive(Clone, Debug)]
pub(super) struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub(super) fn tokens(file: &str, input: &str) -> Result<Vec<Token>> {
    let error = |span, code, message| fail(file, span, code, message);
    if input.len() > MAX_INPUT || input.contains('\r') || input.contains('\0') {
        return Err(error(
            Span { line: 1, column: 1 },
            "input",
            "expected bounded UTF-8 LF input; convert CRLF explicitly",
        ));
    }
    let bytes = input.as_bytes();
    let mut offset = 0;
    let mut line = 1;
    let mut column = 1;
    let mut result = Vec::new();
    while offset < bytes.len() {
        if result.len() >= MAX_VALUES {
            return Err(error(
                Span { line, column },
                "limit",
                "token limit exceeded",
            ));
        }
        let start = offset;
        let span = Span { line, column };
        let c = bytes[offset];
        if c.is_ascii_whitespace() {
            offset += 1;
        } else if input[offset..].starts_with("//") {
            offset += 2;
            while offset < bytes.len() && bytes[offset] != b'\n' {
                offset += 1;
            }
            result.push(Token {
                span,
                kind: TokenKind::Comment(input[start..offset].into()),
            });
        } else if input[offset..].starts_with("\"\"\"") {
            offset += 3;
            if bytes.get(offset) != Some(&b'\n') {
                return Err(error(span, "literal", "raw literal must start with LF"));
            }
            offset += 1;
            let body_start = offset;
            let mut close = None;
            while offset < bytes.len() {
                let line_start = offset;
                while bytes.get(offset) == Some(&b' ') {
                    offset += 1;
                }
                if input[offset..].starts_with("\"\"\"") {
                    close = Some((line_start, offset));
                    break;
                }
                while offset < bytes.len() && bytes[offset] != b'\n' {
                    offset += 1;
                }
                if offset < bytes.len() {
                    offset += 1;
                }
            }
            let (body_end, delimiter) =
                close.ok_or_else(|| error(span, "literal", "unterminated raw literal"))?;
            let prefix = &input[body_end..delimiter];
            let mut decoded = String::new();
            for body_line in input[body_start..body_end].split_inclusive('\n') {
                if body_line == "\n" {
                    decoded.push('\n');
                } else {
                    decoded.push_str(body_line.strip_prefix(prefix).ok_or_else(|| {
                        error(span, "literal", "raw line lacks closing indentation prefix")
                    })?);
                }
            }
            offset = delimiter + 3;
            if bytes.get(offset).is_some_and(|c| !b";,]} \n\t".contains(c)) {
                return Err(error(
                    span,
                    "literal",
                    "raw closing delimiter must stand alone",
                ));
            }
            result.push(Token {
                span,
                kind: TokenKind::Raw(decoded),
            });
        } else if c == b'"' {
            offset += 1;
            loop {
                let next = *bytes
                    .get(offset)
                    .ok_or_else(|| error(span, "string", "unterminated string"))?;
                offset += 1;
                if next == b'"' {
                    break;
                }
                if next == b'\\' {
                    if offset >= bytes.len() {
                        return Err(error(span, "string", "unterminated escape"));
                    }
                    offset += 1;
                }
                if next < 0x20 {
                    return Err(error(span, "string", "control byte in quoted string"));
                }
            }
            let value = serde_json::from_str(&input[start..offset])
                .map_err(|_| error(span, "string", "invalid JSON string escape"))?;
            result.push(Token {
                span,
                kind: TokenKind::String(value),
            });
        } else if c.is_ascii_digit() {
            while offset < bytes.len() && bytes[offset].is_ascii_digit() {
                offset += 1;
            }
            if offset - start > 20 {
                return Err(error(span, "number", "integer exceeds u64"));
            }
            let n = input[start..offset]
                .parse()
                .map_err(|_| error(span, "number", "integer exceeds u64"))?;
            result.push(Token {
                span,
                kind: TokenKind::Number(n),
            });
        } else if c.is_ascii_alphabetic() || c == b'_' {
            while offset < bytes.len()
                && (bytes[offset].is_ascii_alphanumeric() || bytes[offset] == b'_')
            {
                offset += 1;
            }
            if offset - start > 128 {
                return Err(error(span, "name", "identifier exceeds 128 bytes"));
            }
            result.push(Token {
                span,
                kind: TokenKind::Word(input[start..offset].into()),
            });
        } else if b"{}[]();=:,.".contains(&c) {
            offset += 1;
            result.push(Token {
                span,
                kind: TokenKind::Symbol(char::from(c)),
            });
        } else {
            return Err(error(span, "syntax", "unsupported token"));
        }
        for c in input[start..offset].chars() {
            if c == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
    }
    result.push(Token {
        kind: TokenKind::End,
        span: Span { line, column },
    });
    Ok(result)
}

struct Parser<'a> {
    file: &'a str,
    tokens: Vec<Token>,
    position: usize,
    values: usize,
}
impl Parser<'_> {
    fn token(&self) -> &Token {
        &self.tokens[self.position]
    }
    fn error(&self, code: &str, message: &str) -> super::Diagnostic {
        fail(self.file, self.token().span, code, message)
    }
    fn take(&mut self) -> Token {
        let token = self.tokens[self.position].clone();
        if token.kind != TokenKind::End {
            self.position += 1;
        }
        token
    }
    fn symbol(&mut self, symbol: char) -> bool {
        if self.token().kind == TokenKind::Symbol(symbol) {
            self.position += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, symbol: char) -> Result<()> {
        if self.symbol(symbol) {
            Ok(())
        } else {
            Err(self.error("syntax", "expected punctuation"))
        }
    }
    fn word(&mut self) -> Result<String> {
        match &self.token().kind {
            TokenKind::Word(value) => {
                let value = value.clone();
                self.position += 1;
                Ok(value)
            }
            _ => Err(self.error("syntax", "expected identifier")),
        }
    }
    fn keyword(&mut self, word: &str) -> Result<()> {
        if self.token().kind == TokenKind::Word(word.into()) {
            self.position += 1;
            Ok(())
        } else {
            Err(self.error("syntax", "expected keyword"))
        }
    }
    fn string(&mut self) -> Result<String> {
        match &self.token().kind {
            TokenKind::String(value) => {
                let value = value.clone();
                self.position += 1;
                Ok(value)
            }
            _ => Err(self.error("syntax", "expected quoted string")),
        }
    }
    fn block(&mut self, depth: usize) -> Result<Block> {
        if depth > MAX_DEPTH {
            return Err(self.error("limit", "nesting exceeds 64"));
        }
        self.expect('{')?;
        let mut block = Block::default();
        while !self.symbol('}') {
            let span = self.token().span;
            let quoted = matches!(self.token().kind, TokenKind::String(_));
            let name = if quoted { self.string()? } else { self.word()? };
            if !quoted && name == "export" {
                let kind = self.word()?;
                let name = self.string()?;
                self.expect('=')?;
                let path = self.string()?;
                self.expect(';')?;
                if block.exports.iter().any(|e| e.name == name) {
                    return Err(fail(self.file, span, "duplicate", "duplicate export"));
                }
                block.exports.push(Export {
                    kind,
                    name,
                    path,
                    span,
                });
            } else if !quoted && name == "config" {
                let path = self.string()?;
                self.expect('=')?;
                let value = self.value(depth + 1)?;
                self.expect(';')?;
                if block.configs.insert(path, value).is_some() {
                    return Err(fail(self.file, span, "duplicate", "duplicate config path"));
                }
            } else {
                self.expect('=')?;
                let value = self.value(depth + 1)?;
                self.expect(';')?;
                if block.fields.insert(name, value).is_some() {
                    return Err(fail(self.file, span, "duplicate", "duplicate field"));
                }
            }
        }
        Ok(block)
    }
    fn value(&mut self, depth: usize) -> Result<Value> {
        self.values += 1;
        if depth > MAX_DEPTH || self.values > MAX_VALUES {
            return Err(self.error("limit", "expression depth/count exceeded"));
        }
        let span = self.token().span;
        let kind = match self.token().kind.clone() {
            TokenKind::String(value) => {
                self.take();
                Kind::String(value)
            }
            TokenKind::Number(n) => {
                self.take();
                Kind::Integer(n)
            }
            TokenKind::Symbol('[') => {
                self.take();
                let mut values = Vec::new();
                if !self.symbol(']') {
                    loop {
                        values.push(self.value(depth + 1)?);
                        if self.symbol(']') {
                            break;
                        }
                        self.expect(',')?;
                        if self.symbol(']') {
                            break;
                        }
                    }
                }
                Kind::List(values)
            }
            TokenKind::Symbol('{') => Kind::Block(self.block(depth + 1)?),
            TokenKind::Word(name) => {
                self.take();
                if name == "true" || name == "false" {
                    Kind::Boolean(name == "true")
                } else if matches!(name.as_str(), "text" | "shell") {
                    let token = self.take();
                    match token.kind {
                        TokenKind::String(value) | TokenKind::Raw(value) => {
                            Kind::Tagged(name, value)
                        }
                        _ => {
                            return Err(fail(
                                self.file,
                                token.span,
                                "type",
                                "text/shell requires a literal",
                            ));
                        }
                    }
                } else if self.token().kind == TokenKind::Symbol('{') {
                    Kind::Construct(name, self.block(depth + 1)?)
                } else {
                    let mut reference = vec![name];
                    while self.symbol('.') {
                        reference.push(self.word()?);
                    }
                    if self.symbol('(') {
                        let mut args = Vec::new();
                        if !self.symbol(')') {
                            loop {
                                let named = matches!(self.token().kind, TokenKind::Word(_))
                                    && self
                                        .tokens
                                        .get(self.position + 1)
                                        .is_some_and(|t| t.kind == TokenKind::Symbol(':'));
                                let key = if named {
                                    let key = self.word()?;
                                    self.expect(':')?;
                                    Some(key)
                                } else {
                                    None
                                };
                                if args.first().is_some_and(
                                    |(existing, _): &(Option<String>, Value)| {
                                        existing.is_some() != key.is_some()
                                    },
                                ) || key.as_ref().is_some_and(|key| {
                                    args.iter().any(|(other, _)| other.as_ref() == Some(key))
                                }) {
                                    return Err(self
                                        .error("arguments", "mixed or duplicate named arguments"));
                                }
                                args.push((key, self.value(depth + 1)?));
                                if self.symbol(')') {
                                    break;
                                }
                                self.expect(',')?;
                            }
                        }
                        Kind::Call(reference, args)
                    } else {
                        Kind::Reference(reference)
                    }
                }
            }
            _ => return Err(self.error("syntax", "expected value")),
        };
        Ok(Value { span, kind })
    }
    fn module(mut self) -> Result<Module> {
        self.keyword("language")?;
        if self.take().kind != TokenKind::Number(1) {
            return Err(self.error("version", "unsupported language major; upgrade explicitly"));
        }
        self.expect(';')?;
        let mut module = Module {
            file: self.file.into(),
            namespace: None,
            imports: Vec::new(),
            declarations: BTreeMap::new(),
        };
        while self.token().kind != TokenKind::End {
            let span = self.token().span;
            let kind = self.word()?;
            if kind == "namespace" {
                if module.namespace.is_some() {
                    return Err(self.error("duplicate", "duplicate namespace"));
                }
                module.namespace = Some(self.string()?);
                self.expect(';')?;
            } else if kind == "import" {
                self.expect('{')?;
                let mut names = vec![self.word()?];
                while self.symbol(',') {
                    let name = self.word()?;
                    if names.contains(&name) {
                        return Err(self.error("duplicate", "duplicate import name"));
                    }
                    names.push(name);
                }
                self.expect('}')?;
                self.keyword("from")?;
                let path = self.string()?;
                self.expect(';')?;
                module.imports.push(Import { names, path, span });
            } else {
                if !matches!(
                    kind.as_str(),
                    "foundation" | "builder" | "set" | "target" | "package" | "library"
                ) {
                    return Err(fail(
                        self.file,
                        span,
                        "declaration",
                        "unsupported declaration",
                    ));
                }
                let name = if kind == "target" {
                    self.string()?
                } else {
                    self.word()?
                };
                let mut parameters = BTreeMap::new();
                if matches!(kind.as_str(), "package" | "library") {
                    self.expect('(')?;
                    if !self.symbol(')') {
                        loop {
                            let parameter = self.word()?;
                            self.expect(':')?;
                            let ty = self.word()?;
                            if !matches!(ty.as_str(), "Builder" | "Foundation")
                                || parameters.insert(parameter, ty).is_some()
                            {
                                return Err(
                                    self.error("type", "unknown type or duplicate parameter")
                                );
                            }
                            if self.symbol(')') {
                                break;
                            }
                            self.expect(',')?;
                        }
                    }
                }
                let block = self.block(1)?;
                if module
                    .declarations
                    .insert(
                        name.clone(),
                        Declaration {
                            kind,
                            name,
                            span,
                            parameters,
                            block,
                        },
                    )
                    .is_some()
                {
                    return Err(fail(self.file, span, "duplicate", "duplicate declaration"));
                }
            }
        }
        Ok(module)
    }
}

pub fn parse(file: &str, input: &str) -> Result<Module> {
    let tokens = tokens(file, input)?
        .into_iter()
        .filter(|t| !matches!(t.kind, TokenKind::Comment(_)))
        .collect();
    Parser {
        file,
        tokens,
        position: 0,
        values: 0,
    }
    .module()
}
