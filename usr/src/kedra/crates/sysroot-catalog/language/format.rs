use super::parser::{TokenKind, tokens};
use super::{Result, parse};

/// Canonical whitespace with opaque decoded literals and preserved comments.
pub fn format(file: &str, input: &str) -> Result<String> {
    parse(file, input)?;
    let tokens = tokens(file, input)?;
    let mut out = String::new();
    let mut indent = 0usize;
    let mut brackets = Vec::new();
    let mut line_start = true;
    let mut previous = TokenKind::End;
    for (index, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::End {
            break;
        }
        if matches!(token.kind, TokenKind::Symbol('}')) {
            indent = indent.saturating_sub(1);
            if !line_start {
                out.push('\n');
                line_start = true;
            }
        }
        if line_start {
            out.push_str(&"    ".repeat(indent));
            line_start = false;
        }
        let needs_space = !matches!(
            token.kind,
            TokenKind::Symbol(')' | ']' | ';' | ',' | '.' | ':')
        ) && !matches!(
            previous,
            TokenKind::Symbol('(' | '[' | '.') | TokenKind::End
        ) && !out.ends_with([' ', '\n']);
        if needs_space && !matches!(token.kind, TokenKind::Symbol('(')) {
            out.push(' ');
        }
        match &token.kind {
            TokenKind::Word(value) | TokenKind::Comment(value) => out.push_str(value),
            TokenKind::String(value) => out
                .push_str(&serde_json::to_string(value).map_err(|_| {
                    super::fail(file, token.span, "format", "cannot encode string")
                })?),
            TokenKind::Raw(value) => {
                if !value.ends_with('\n') || value.contains("\"\"\"") {
                    out.push_str(&serde_json::to_string(value).map_err(|_| {
                        super::fail(file, token.span, "format", "cannot encode literal")
                    })?);
                } else {
                    out.push_str("\"\"\"\n");
                    let prefix = "    ".repeat(indent + 1);
                    for line in value.split_inclusive('\n') {
                        if line != "\n" {
                            out.push_str(&prefix);
                        }
                        out.push_str(line);
                    }
                    out.push_str(&prefix);
                    out.push_str("\"\"\"");
                }
            }
            TokenKind::Number(value) => out.push_str(&value.to_string()),
            TokenKind::Symbol(symbol) => {
                out.push(*symbol);
                match symbol {
                    '{' => {
                        indent += 1;
                        brackets.push('{');
                        out.push('\n');
                        line_start = true;
                    }
                    '}' => {
                        brackets.pop();
                    }
                    '[' | '(' => brackets.push(*symbol),
                    ']' | ')' => {
                        brackets.pop();
                    }
                    ';' => {
                        out.push('\n');
                        line_start = true;
                    }
                    '=' | ':' | ',' => out.push(' '),
                    _ => {}
                }
            }
            TokenKind::End => {}
        }
        if matches!(token.kind, TokenKind::Comment(_)) {
            out.push('\n');
            line_start = true;
        }
        if token.kind == TokenKind::Symbol('}')
            && tokens.get(index + 1).is_some_and(|t| {
                matches!(&t.kind, TokenKind::Word(word) if word != "from")
                    || matches!(t.kind, TokenKind::Comment(_))
            })
        {
            out.push('\n');
            line_start = true;
        }
        previous = if line_start {
            TokenKind::End
        } else {
            token.kind.clone()
        };
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    parse(file, &out)?;
    let normalized = |tokens: Vec<super::parser::Token>| {
        tokens
            .into_iter()
            .map(|t| match t.kind {
                TokenKind::Raw(value) => TokenKind::String(value),
                other => other,
            })
            .collect::<Vec<_>>()
    };
    if normalized(tokens.clone()) != normalized(super::parser::tokens(file, &out)?) {
        return Err(super::fail(
            file,
            super::Span { line: 1, column: 1 },
            "format",
            "formatter changed declarations",
        ));
    }
    Ok(out)
}
