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
    if input.len() > MAX_INPUT || input.contains('\r') || input.contains('\0') {
        return Err(fail(
            file,
            Span { line: 1, column: 1 },
            "input",
            "expected bounded UTF-8 LF input; convert CRLF explicitly",
        ));
    }
    let mut lexer = Lexer {
        file,
        input,
        offset: 0,
        line: 1,
        column: 1,
    };
    let mut result = Vec::new();
    while lexer.offset < input.len() {
        if result.len() >= MAX_VALUES {
            return Err(lexer.error("limit", "token limit exceeded"));
        }
        let start = lexer.offset;
        let span = lexer.span();
        if let Some(kind) = lexer.lex()? {
            result.push(Token { span, kind });
        }
        lexer.advance_span(start);
    }
    result.push(Token {
        kind: TokenKind::End,
        span: lexer.span(),
    });
    Ok(result)
}

struct Lexer<'a> {
    file: &'a str,
    input: &'a str,
    offset: usize,
    line: usize,
    column: usize,
}
impl<'a> Lexer<'a> {
    fn span(&self) -> Span {
        Span {
            line: self.line,
            column: self.column,
        }
    }
    fn error(&self, code: &str, message: &str) -> super::Diagnostic {
        fail(self.file, self.span(), code, message)
    }
    fn take_while(&mut self, keep: impl Fn(&u8) -> bool) -> &'a str {
        let start = self.offset;
        while self.input.as_bytes().get(self.offset).is_some_and(&keep) {
            self.offset += 1;
        }
        &self.input[start..self.offset]
    }
    fn advance_span(&mut self, start: usize) {
        for c in self.input[start..self.offset].chars() {
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
    }
    fn lex(&mut self) -> Result<Option<TokenKind>> {
        let c = self.input.as_bytes()[self.offset];
        if c.is_ascii_whitespace() {
            self.offset += 1;
            return Ok(None);
        }
        let rest = &self.input[self.offset..];
        let kind = if rest.starts_with("//") {
            TokenKind::Comment(self.take_while(|&byte| byte != b'\n').into())
        } else if rest.starts_with("\"\"\"") {
            self.raw()?
        } else if c == b'"' {
            self.quoted()?
        } else if c.is_ascii_digit() {
            self.number()?
        } else if c.is_ascii_alphabetic() || c == b'_' {
            self.word()?
        } else if b"{}[]();=:,.".contains(&c) {
            self.offset += 1;
            TokenKind::Symbol(char::from(c))
        } else {
            return Err(self.error("syntax", "unsupported token"));
        };
        Ok(Some(kind))
    }
    fn quoted(&mut self) -> Result<TokenKind> {
        let bytes = self.input.as_bytes();
        let start = self.offset;
        self.offset += 1;
        loop {
            let next = *bytes
                .get(self.offset)
                .ok_or_else(|| self.error("string", "unterminated string"))?;
            self.offset += 1;
            if next == b'"' {
                break;
            }
            if next == b'\\' {
                if self.offset >= bytes.len() {
                    return Err(self.error("string", "unterminated escape"));
                }
                self.offset += 1;
            }
            if next < 0x20 {
                return Err(self.error("string", "control byte in quoted string"));
            }
        }
        let value = serde_json::from_str(&self.input[start..self.offset])
            .map_err(|_| self.error("string", "invalid JSON string escape"))?;
        Ok(TokenKind::String(value))
    }
    fn number(&mut self) -> Result<TokenKind> {
        let digits = self.take_while(u8::is_ascii_digit);
        if digits.len() > 20 {
            return Err(self.error("number", "integer exceeds u64"));
        }
        let n = digits
            .parse()
            .map_err(|_| self.error("number", "integer exceeds u64"))?;
        Ok(TokenKind::Number(n))
    }
    fn word(&mut self) -> Result<TokenKind> {
        let word = self.take_while(|&byte| byte.is_ascii_alphanumeric() || byte == b'_');
        if word.len() > 128 {
            return Err(self.error("name", "identifier exceeds 128 bytes"));
        }
        Ok(TokenKind::Word(word.into()))
    }
    fn raw(&mut self) -> Result<TokenKind> {
        let bytes = self.input.as_bytes();
        self.offset += 3;
        if bytes.get(self.offset) != Some(&b'\n') {
            return Err(self.error("literal", "raw literal must start with LF"));
        }
        let body_start = self.offset + 1;
        let (body_end, delimiter) = raw_close(self.input, body_start)
            .ok_or_else(|| self.error("literal", "unterminated raw literal"))?;
        let body = &self.input[body_start..body_end];
        if body.contains("\"\"\"") {
            return Err(self.error("literal", "delimiter in payload requires an escaped string"));
        }
        let decoded = dedent(body, &self.input[body_end..delimiter])
            .ok_or_else(|| self.error("literal", "raw line lacks closing indentation prefix"))?;
        self.offset = delimiter + 3;
        if bytes
            .get(self.offset)
            .is_some_and(|c| !b";,]} \n\t".contains(c))
        {
            return Err(self.error("literal", "raw closing delimiter must stand alone"));
        }
        Ok(TokenKind::Raw(decoded))
    }
}

fn raw_close(input: &str, mut offset: usize) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    while offset < bytes.len() {
        let line_start = offset;
        while bytes.get(offset) == Some(&b' ') {
            offset += 1;
        }
        if input[offset..].starts_with("\"\"\"") {
            return Some((line_start, offset));
        }
        while offset < bytes.len() && bytes[offset] != b'\n' {
            offset += 1;
        }
        if offset < bytes.len() {
            offset += 1;
        }
    }
    None
}

fn dedent(body: &str, prefix: &str) -> Option<String> {
    let mut decoded = String::new();
    for line in body.split_inclusive('\n') {
        if line == "\n" {
            decoded.push('\n');
        } else {
            decoded.push_str(line.strip_prefix(prefix)?);
        }
    }
    Some(decoded)
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
            TokenKind::Symbol('[') => self.list(depth)?,
            TokenKind::Symbol('{') => Kind::Block(self.block(depth + 1)?),
            TokenKind::Word(name) => {
                self.take();
                self.word_value(name, depth)?
            }
            _ => return Err(self.error("syntax", "expected value")),
        };
        Ok(Value { span, kind })
    }
    fn list(&mut self, depth: usize) -> Result<Kind> {
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
        Ok(Kind::List(values))
    }
    fn word_value(&mut self, name: String, depth: usize) -> Result<Kind> {
        if name == "true" || name == "false" {
            Ok(Kind::Boolean(name == "true"))
        } else if matches!(name.as_str(), "text" | "shell") {
            self.tagged(name)
        } else if self.token().kind == TokenKind::Symbol('{') {
            Ok(Kind::Construct(name, self.block(depth + 1)?))
        } else {
            self.reference_or_call(name, depth)
        }
    }
    fn tagged(&mut self, tag: String) -> Result<Kind> {
        let token = self.take();
        match token.kind {
            TokenKind::String(value) | TokenKind::Raw(value) => Ok(Kind::Tagged(tag, value)),
            _ => Err(fail(
                self.file,
                token.span,
                "type",
                "text/shell requires a literal",
            )),
        }
    }
    fn reference_or_call(&mut self, name: String, depth: usize) -> Result<Kind> {
        let mut reference = vec![name];
        while self.symbol('.') {
            reference.push(self.word()?);
        }
        if self.symbol('(') {
            Ok(Kind::Call(reference, self.arguments(depth)?))
        } else {
            Ok(Kind::Reference(reference))
        }
    }
    fn arguments(&mut self, depth: usize) -> Result<Vec<(Option<String>, Value)>> {
        let mut args = Vec::new();
        if !self.symbol(')') {
            loop {
                let key = self.argument_name()?;
                if Self::mixed_or_duplicate(&args, key.as_deref()) {
                    return Err(self.error("arguments", "mixed or duplicate named arguments"));
                }
                args.push((key, self.value(depth + 1)?));
                if self.symbol(')') {
                    break;
                }
                self.expect(',')?;
            }
        }
        Ok(args)
    }
    fn argument_name(&mut self) -> Result<Option<String>> {
        let named = matches!(self.token().kind, TokenKind::Word(_))
            && self
                .tokens
                .get(self.position + 1)
                .is_some_and(|t| t.kind == TokenKind::Symbol(':'));
        if !named {
            return Ok(None);
        }
        let key = self.word()?;
        self.expect(':')?;
        Ok(Some(key))
    }
    fn mixed_or_duplicate(args: &[(Option<String>, Value)], key: Option<&str>) -> bool {
        args.first()
            .is_some_and(|(existing, _)| existing.is_some() != key.is_some())
            || key.is_some_and(|key| args.iter().any(|(other, _)| other.as_deref() == Some(key)))
    }
    fn module(mut self) -> Result<Module> {
        self.header()?;
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
                module.imports.push(self.import(span)?);
            } else {
                let declaration = self.declaration(kind, span)?;
                if module
                    .declarations
                    .insert(declaration.name.clone(), declaration)
                    .is_some()
                {
                    return Err(fail(self.file, span, "duplicate", "duplicate declaration"));
                }
            }
        }
        Ok(module)
    }
    fn header(&mut self) -> Result<()> {
        self.keyword("language")?;
        if self.take().kind != TokenKind::Number(1) {
            return Err(self.error("version", "unsupported language major; upgrade explicitly"));
        }
        self.expect(';')
    }
    fn import(&mut self, span: Span) -> Result<Import> {
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
        Ok(Import { names, path, span })
    }
    fn declaration(&mut self, kind: String, span: Span) -> Result<Declaration> {
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
        let parameters = if matches!(kind.as_str(), "package" | "library") {
            self.parameters()?
        } else {
            BTreeMap::new()
        };
        let block = self.block(1)?;
        Ok(Declaration {
            kind,
            name,
            span,
            parameters,
            block,
        })
    }
    fn parameters(&mut self) -> Result<BTreeMap<String, String>> {
        self.expect('(')?;
        let mut parameters = BTreeMap::new();
        if !self.symbol(')') {
            loop {
                let parameter = self.word()?;
                self.expect(':')?;
                let ty = self.word()?;
                if !matches!(ty.as_str(), "Builder" | "Foundation")
                    || parameters.insert(parameter, ty).is_some()
                {
                    return Err(self.error("type", "unknown type or duplicate parameter"));
                }
                if self.symbol(')') {
                    break;
                }
                self.expect(',')?;
            }
        }
        Ok(parameters)
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
