use super::parser::{Token, TokenKind, tokens};
use super::{Result, parse};

/// Canonical whitespace with opaque decoded literals and preserved comments.
pub fn format(file: &str, input: &str) -> Result<String> {
    parse(file, input)?;
    let tokens = tokens(file, input)?;
    let out = Printer::print(file, &tokens)?;
    verify(file, tokens, &out)?;
    Ok(out)
}

struct Printer {
    out: String,
    indent: usize,
    brackets: Vec<char>,
    line_start: bool,
    previous: TokenKind,
}

impl Printer {
    fn print(file: &str, tokens: &[Token]) -> Result<String> {
        let mut printer = Printer {
            out: String::new(),
            indent: 0,
            brackets: Vec::new(),
            line_start: true,
            previous: TokenKind::End,
        };
        for (index, token) in tokens.iter().enumerate() {
            if token.kind == TokenKind::End {
                break;
            }
            printer.token(file, token, tokens.get(index + 1))?;
        }
        if !printer.out.ends_with('\n') {
            printer.out.push('\n');
        }
        Ok(printer.out)
    }

    fn token(&mut self, file: &str, token: &Token, next: Option<&Token>) -> Result<()> {
        let closes_block = token.kind == TokenKind::Symbol('}');
        if closes_block {
            self.close_block();
        }
        self.start_line();
        self.space_before(&token.kind);
        self.write(file, token)?;
        if matches!(token.kind, TokenKind::Comment(_)) {
            self.break_line();
        }
        if closes_block && next.is_some_and(own_line_after_block) {
            self.break_line();
        }
        self.previous = if self.line_start {
            TokenKind::End
        } else {
            token.kind.clone()
        };
        Ok(())
    }

    fn close_block(&mut self) {
        self.indent = self.indent.saturating_sub(1);
        if !self.line_start {
            self.break_line();
        }
    }

    fn start_line(&mut self) {
        if self.line_start {
            self.out.push_str(&"    ".repeat(self.indent));
            self.line_start = false;
        }
    }

    fn break_line(&mut self) {
        self.out.push('\n');
        self.line_start = true;
    }

    fn space_before(&mut self, kind: &TokenKind) {
        let needs_space = !matches!(kind, TokenKind::Symbol(')' | ']' | ';' | ',' | '.' | ':'))
            && !matches!(
                self.previous,
                TokenKind::Symbol('(' | '[' | '.') | TokenKind::End
            )
            && !self.out.ends_with([' ', '\n']);
        if needs_space && !matches!(kind, TokenKind::Symbol('(')) {
            self.out.push(' ');
        }
    }

    fn write(&mut self, file: &str, token: &Token) -> Result<()> {
        match &token.kind {
            TokenKind::Word(value) | TokenKind::Comment(value) => self.out.push_str(value),
            TokenKind::String(value) => {
                self.out
                    .push_str(&quoted(file, token, value, "cannot encode string")?);
            }
            TokenKind::Raw(value) => self.raw(file, token, value)?,
            TokenKind::Number(value) => self.out.push_str(&value.to_string()),
            TokenKind::Symbol(symbol) => self.symbol(*symbol),
            TokenKind::End => {}
        }
        Ok(())
    }

    // A raw literal keeps its triple-quoted block form, indented one level
    // deeper, unless only a quoted string can carry its bytes.
    fn raw(&mut self, file: &str, token: &Token, value: &str) -> Result<()> {
        if !value.ends_with('\n') || value.contains("\"\"\"") {
            self.out
                .push_str(&quoted(file, token, value, "cannot encode literal")?);
            return Ok(());
        }
        self.out.push_str("\"\"\"\n");
        let prefix = "    ".repeat(self.indent + 1);
        for line in value.split_inclusive('\n') {
            if line != "\n" {
                self.out.push_str(&prefix);
            }
            self.out.push_str(line);
        }
        self.out.push_str(&prefix);
        self.out.push_str("\"\"\"");
        Ok(())
    }

    fn symbol(&mut self, symbol: char) {
        self.out.push(symbol);
        match symbol {
            '{' => {
                self.indent += 1;
                self.brackets.push('{');
                self.break_line();
            }
            '}' | ']' | ')' => {
                self.brackets.pop();
            }
            '[' | '(' => self.brackets.push(symbol),
            ';' => self.break_line(),
            '=' | ':' | ',' => self.out.push(' '),
            _ => {}
        }
    }
}

fn quoted(file: &str, token: &Token, value: &str, message: &str) -> Result<String> {
    serde_json::to_string(value).map_err(|_| super::fail(file, token.span, "format", message))
}

fn own_line_after_block(next: &Token) -> bool {
    matches!(&next.kind, TokenKind::Word(word) if word != "from")
        || matches!(next.kind, TokenKind::Comment(_))
}

fn verify(file: &str, tokens: Vec<Token>, out: &str) -> Result<()> {
    parse(file, out)?;
    if normalized(tokens) != normalized(super::parser::tokens(file, out)?) {
        return Err(super::fail(
            file,
            super::Span { line: 1, column: 1 },
            "format",
            "formatter changed declarations",
        ));
    }
    Ok(())
}

fn normalized(tokens: Vec<Token>) -> Vec<TokenKind> {
    tokens
        .into_iter()
        .map(|t| match t.kind {
            TokenKind::Raw(value) => TokenKind::String(value),
            other => other,
        })
        .collect()
}
