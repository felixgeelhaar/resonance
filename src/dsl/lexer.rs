//! Lexer for the Resonance DSL.
//!
//! Converts source text into a stream of [`Token`]s.

use super::error::CompileError;
use super::token::{NoteToken, StepToken, Token, TokenKind};

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    pending: Vec<Token>,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            pending: Vec::new(),
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, CompileError> {
        let mut tokens = Vec::new();

        loop {
            // Drain pending tokens first
            if !self.pending.is_empty() {
                tokens.append(&mut self.pending);
                continue;
            }

            self.skip_whitespace();
            self.skip_comment();
            self.skip_whitespace();

            if self.is_at_end() {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    line: self.line,
                    col: self.col,
                });
                break;
            }

            let ch = self.peek();

            if ch == '\n' {
                tokens.push(Token {
                    kind: TokenKind::Newline,
                    line: self.line,
                    col: self.col,
                });
                self.advance();
                self.line += 1;
                self.col = 1;
                continue;
            }

            let token = match ch {
                '{' => self.single_char(TokenKind::LBrace),
                '}' => self.single_char(TokenKind::RBrace),
                '(' => self.single_char(TokenKind::LParen),
                ')' => self.single_char(TokenKind::RParen),
                ':' => self.single_char(TokenKind::Colon),
                ',' => self.single_char(TokenKind::Comma),
                '=' => self.single_char(TokenKind::Eq),
                '"' => self.lex_string()?,
                '[' => self.lex_bracket_content()?,
                '|' => self.lex_pipe()?,
                '-' => self.lex_arrow_or_number()?,
                '.' if self.peek_next() == Some('.') => {
                    let line = self.line;
                    let col = self.col;
                    self.advance();
                    self.advance();
                    Token {
                        kind: TokenKind::DotDot,
                        line,
                        col,
                    }
                }
                '.' if self.peek_next().is_some_and(|c| c.is_ascii_digit()) => self.lex_number()?,
                '.' => self.single_char(TokenKind::Dot),
                '0'..='9' => self.lex_number()?,
                'a'..='z' | 'A'..='Z' | '_' => self.lex_ident_or_keyword(),
                _ => {
                    return Err(CompileError::lex(
                        format!("unexpected character: '{ch}'"),
                        self.line,
                        self.col,
                    ));
                }
            };

            tokens.push(token);
        }

        Ok(tokens)
    }

    fn peek(&self) -> char {
        self.chars[self.pos]
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> char {
        let ch = self.chars[self.pos];
        self.pos += 1;
        if ch != '\n' {
            self.col += 1;
        }
        ch
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            let ch = self.peek();
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        if !self.is_at_end() && self.peek() == '/' && self.peek_next() == Some('/') {
            while !self.is_at_end() && self.peek() != '\n' {
                self.advance();
            }
        }
    }

    fn single_char(&mut self, kind: TokenKind) -> Token {
        let line = self.line;
        let col = self.col;
        self.advance();
        Token { kind, line, col }
    }

    fn lex_string(&mut self) -> Result<Token, CompileError> {
        let line = self.line;
        let col = self.col;
        self.advance(); // consume opening '"'
        let mut s = String::new();
        while !self.is_at_end() && self.peek() != '"' {
            s.push(self.advance());
        }
        if self.is_at_end() {
            return Err(CompileError::lex("unclosed string literal", line, col));
        }
        self.advance(); // consume closing '"'
        Ok(Token {
            kind: TokenKind::Ident(s),
            line,
            col,
        })
    }

    fn lex_pipe(&mut self) -> Result<Token, CompileError> {
        let line = self.line;
        let col = self.col;
        self.advance(); // consume '|'
        if !self.is_at_end() && self.peek() == '>' {
            self.advance();
            Ok(Token {
                kind: TokenKind::Pipe,
                line,
                col,
            })
        } else {
            Err(CompileError::lex("expected '>' after '|'", line, col))
        }
    }

    fn lex_arrow_or_number(&mut self) -> Result<Token, CompileError> {
        let line = self.line;
        let col = self.col;

        if self.peek_next() == Some('>') {
            self.advance();
            self.advance();
            return Ok(Token {
                kind: TokenKind::Arrow,
                line,
                col,
            });
        }

        // Check if this is a negative number
        if self
            .peek_next()
            .is_some_and(|c| c.is_ascii_digit() || c == '.')
        {
            return self.lex_number();
        }

        Err(CompileError::lex("unexpected '-'", line, col))
    }

    fn lex_number(&mut self) -> Result<Token, CompileError> {
        let line = self.line;
        let col = self.col;
        let mut s = String::new();

        if !self.is_at_end() && self.peek() == '-' {
            s.push(self.advance());
        }

        while !self.is_at_end() && self.peek().is_ascii_digit() {
            s.push(self.advance());
        }

        let is_float = !self.is_at_end() && self.peek() == '.' && self.peek_next() != Some('.');
        if is_float {
            s.push(self.advance()); // consume '.'
            while !self.is_at_end() && self.peek().is_ascii_digit() {
                s.push(self.advance());
            }
        }

        // Check for fraction like 1/8
        if !self.is_at_end() && self.peek() == '/' {
            let saved_pos = self.pos;
            let saved_col = self.col;
            self.advance(); // consume '/'
            let mut denom = String::new();
            while !self.is_at_end() && self.peek().is_ascii_digit() {
                denom.push(self.advance());
            }
            if !denom.is_empty() {
                let num: f64 = s.parse().unwrap_or(0.0);
                let den: f64 = denom.parse().unwrap_or(1.0);
                return Ok(Token {
                    kind: TokenKind::Number(num / den),
                    line,
                    col,
                });
            }
            // Not a fraction, restore
            self.pos = saved_pos;
            self.col = saved_col;
        }

        if is_float {
            let val: f64 = s
                .parse()
                .map_err(|_| CompileError::lex(format!("invalid number: {s}"), line, col))?;
            Ok(Token {
                kind: TokenKind::Number(val),
                line,
                col,
            })
        } else {
            let val: f64 = s
                .parse()
                .map_err(|_| CompileError::lex(format!("invalid number: {s}"), line, col))?;
            if val >= 0.0 && val == (val as u64) as f64 {
                Ok(Token {
                    kind: TokenKind::Integer(val as u64),
                    line,
                    col,
                })
            } else {
                Ok(Token {
                    kind: TokenKind::Number(val),
                    line,
                    col,
                })
            }
        }
    }

    fn lex_ident_or_keyword(&mut self) -> Token {
        let line = self.line;
        let col = self.col;
        let mut s = String::new();

        while !self.is_at_end()
            && (self.peek().is_ascii_alphanumeric() || self.peek() == '_' || self.peek() == '#')
        {
            s.push(self.advance());
        }

        let kind = match s.as_str() {
            "tempo" => TokenKind::Tempo,
            "track" => TokenKind::Track,
            "section" => TokenKind::Section,
            "macro" => TokenKind::Macro,
            "map" => TokenKind::Map,
            "kit" => TokenKind::Kit,
            "bass" => TokenKind::Bass,
            "poly" => TokenKind::Poly,
            "pluck" => TokenKind::Pluck,
            "noise" => TokenKind::Noise,
            "vel" => TokenKind::Vel,
            "bars" => TokenKind::Bars,
            _ => TokenKind::Ident(s),
        };

        Token { kind, line, col }
    }

    /// Lex content inside brackets `[...]`.
    fn lex_bracket_content(&mut self) -> Result<Token, CompileError> {
        let line = self.line;
        let col = self.col;
        self.advance(); // consume '['

        let content = self.collect_bracket_content()?;
        let trimmed = content.trim();

        if trimmed.is_empty() {
            return Ok(Token {
                kind: TokenKind::LBracket,
                line,
                col,
            });
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();

        // [N bars] — section length specifier
        if parts.len() == 2 && parts[1] == "bars" {
            if let Ok(n) = parts[0].parse::<u64>() {
                // Push Integer(n), Bars, RBracket as pending
                self.pending.push(Token {
                    kind: TokenKind::Integer(n),
                    line,
                    col,
                });
                self.pending.push(Token {
                    kind: TokenKind::Bars,
                    line,
                    col,
                });
                self.pending.push(Token {
                    kind: TokenKind::RBracket,
                    line,
                    col,
                });
                return Ok(Token {
                    kind: TokenKind::LBracket,
                    line,
                    col,
                });
            }
        }

        // Step pattern (only X, x, .)
        let all_steps = parts
            .iter()
            .all(|p| p.len() == 1 && matches!(p.chars().next(), Some('X' | 'x' | '.')));

        if all_steps && !parts.is_empty() {
            let steps: Vec<StepToken> = parts
                .iter()
                .map(|p| match p.chars().next().unwrap() {
                    'X' => StepToken::Accent,
                    'x' => StepToken::Ghost,
                    '.' => StepToken::Rest,
                    _ => unreachable!(),
                })
                .collect();
            return Ok(Token {
                kind: TokenKind::StepPattern(steps),
                line,
                col,
            });
        }

        // Note pattern (contains note names)
        let has_notes = parts.iter().any(|p| is_note_name(p));
        if has_notes {
            let notes: Vec<NoteToken> = parts
                .iter()
                .map(|p| {
                    if *p == "." {
                        NoteToken::Rest
                    } else {
                        NoteToken::Note(p.to_string())
                    }
                })
                .collect();
            return Ok(Token {
                kind: TokenKind::NotePattern(notes),
                line,
                col,
            });
        }

        // Numeric array — parse as velocity values
        let nums: Result<Vec<f64>, _> = parts
            .iter()
            .map(|p| {
                if *p == "." {
                    Ok(0.0)
                } else {
                    p.parse::<f64>().map_err(|_| {
                        CompileError::lex(
                            format!("expected number in bracket, got '{p}'"),
                            line,
                            col,
                        )
                    })
                }
            })
            .collect();
        let numbers = nums?;

        let steps: Vec<StepToken> = numbers
            .iter()
            .map(|&v| {
                if v == 0.0 {
                    StepToken::Rest
                } else {
                    StepToken::Hit
                }
            })
            .collect();

        Ok(Token {
            kind: TokenKind::StepPattern(steps),
            line,
            col,
        })
    }

    fn collect_bracket_content(&mut self) -> Result<String, CompileError> {
        let mut content = String::new();
        let mut depth = 1;
        let start_line = self.line;
        let start_col = self.col;

        while !self.is_at_end() {
            let ch = self.peek();
            if ch == '[' {
                depth += 1;
            } else if ch == ']' {
                depth -= 1;
                if depth == 0 {
                    self.advance(); // consume ']'
                    return Ok(content);
                }
            }
            if ch == '\n' {
                self.line += 1;
                self.col = 0;
            }
            content.push(self.advance());
        }

        Err(CompileError::lex("unclosed bracket", start_line, start_col))
    }
}

fn is_note_name(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return false;
    }
    if !matches!(chars[0], 'A'..='G') {
        return false;
    }
    let mut i = 1;
    if i < chars.len() && (chars[i] == '#' || chars[i] == 'b') {
        i += 1;
    }
    if i >= chars.len() || !chars[i].is_ascii_digit() {
        return false;
    }
    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_tempo() {
        let mut lexer = Lexer::new("tempo 128");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Tempo);
        assert_eq!(tokens[1].kind, TokenKind::Integer(128));
    }

    #[test]
    fn lex_track_keyword() {
        let mut lexer = Lexer::new("track drums");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Track);
        assert_eq!(tokens[1].kind, TokenKind::Ident("drums".to_string()));
    }

    #[test]
    fn lex_braces() {
        let mut lexer = Lexer::new("{ }");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::LBrace);
        assert_eq!(tokens[1].kind, TokenKind::RBrace);
    }

    #[test]
    fn lex_step_pattern() {
        let mut lexer = Lexer::new("[X . . x . X . .]");
        let tokens = lexer.tokenize().unwrap();
        match &tokens[0].kind {
            TokenKind::StepPattern(steps) => {
                assert_eq!(steps.len(), 8);
                assert_eq!(steps[0], StepToken::Accent);
                assert_eq!(steps[1], StepToken::Rest);
                assert_eq!(steps[3], StepToken::Ghost);
                assert_eq!(steps[5], StepToken::Accent);
            }
            other => panic!("expected StepPattern, got {other:?}"),
        }
    }

    #[test]
    fn lex_note_pattern() {
        let mut lexer = Lexer::new("[C2 . . C2 . . Eb2 .]");
        let tokens = lexer.tokenize().unwrap();
        match &tokens[0].kind {
            TokenKind::NotePattern(notes) => {
                assert_eq!(notes.len(), 8);
                assert_eq!(notes[0], NoteToken::Note("C2".to_string()));
                assert_eq!(notes[1], NoteToken::Rest);
                assert_eq!(notes[6], NoteToken::Note("Eb2".to_string()));
            }
            other => panic!("expected NotePattern, got {other:?}"),
        }
    }

    #[test]
    fn lex_section_bars() {
        let mut lexer = Lexer::new("[2 bars]");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::LBracket);
        assert_eq!(tokens[1].kind, TokenKind::Integer(2));
        assert_eq!(tokens[2].kind, TokenKind::Bars);
        assert_eq!(tokens[3].kind, TokenKind::RBracket);
    }

    #[test]
    fn lex_pipe() {
        let mut lexer = Lexer::new("|>");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Pipe);
    }

    #[test]
    fn lex_float() {
        let mut lexer = Lexer::new("0.95");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Number(0.95));
    }

    #[test]
    fn lex_fraction() {
        let mut lexer = Lexer::new("1/8");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Number(0.125));
    }

    #[test]
    fn lex_keywords() {
        let mut lexer = Lexer::new("kit bass poly pluck noise vel bars section macro map");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Kit);
        assert_eq!(tokens[1].kind, TokenKind::Bass);
        assert_eq!(tokens[2].kind, TokenKind::Poly);
        assert_eq!(tokens[3].kind, TokenKind::Pluck);
        assert_eq!(tokens[4].kind, TokenKind::Noise);
        assert_eq!(tokens[5].kind, TokenKind::Vel);
        assert_eq!(tokens[6].kind, TokenKind::Bars);
        assert_eq!(tokens[7].kind, TokenKind::Section);
        assert_eq!(tokens[8].kind, TokenKind::Macro);
        assert_eq!(tokens[9].kind, TokenKind::Map);
    }

    #[test]
    fn lex_line_tracking() {
        let mut lexer = Lexer::new("tempo 128\ntrack drums");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[2].line, 1); // Newline token
        assert_eq!(tokens[3].line, 2); // track
    }

    #[test]
    fn lex_comment() {
        let mut lexer = Lexer::new("tempo 128 // this is a comment\ntrack drums");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Tempo);
        assert_eq!(tokens[1].kind, TokenKind::Integer(128));
        assert_eq!(tokens[2].kind, TokenKind::Newline);
        assert_eq!(tokens[3].kind, TokenKind::Track);
    }

    #[test]
    fn lex_eq_and_dot() {
        let mut lexer = Lexer::new("x = foo.bar");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Ident("x".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::Eq);
        assert_eq!(tokens[2].kind, TokenKind::Ident("foo".to_string()));
        assert_eq!(tokens[3].kind, TokenKind::Dot);
        assert_eq!(tokens[4].kind, TokenKind::Ident("bar".to_string()));
    }

    #[test]
    fn lex_error_on_unexpected_char() {
        let mut lexer = Lexer::new("tempo 128 @");
        let result = lexer.tokenize();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind, super::super::error::ErrorKind::LexError);
    }

    #[test]
    fn lex_unclosed_bracket_error() {
        let mut lexer = Lexer::new("[X . .");
        let result = lexer.tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn lex_note_name_detection() {
        assert!(is_note_name("C2"));
        assert!(is_note_name("Eb4"));
        assert!(is_note_name("F#3"));
        assert!(is_note_name("A0"));
        assert!(!is_note_name("X"));
        assert!(!is_note_name("foo"));
        assert!(!is_note_name("."));
        assert!(!is_note_name("128"));
    }

    #[test]
    fn lex_functional_syntax() {
        let src = r#"drums = kit("default") |> kick.pattern("X..x")"#;
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Ident("drums".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::Eq);
        assert_eq!(tokens[2].kind, TokenKind::Kit);
    }

    #[test]
    fn lex_declarative_section() {
        let src = "section groove [2 bars]";
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Section);
        assert_eq!(tokens[1].kind, TokenKind::Ident("groove".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::LBracket);
        assert_eq!(tokens[3].kind, TokenKind::Integer(2));
        assert_eq!(tokens[4].kind, TokenKind::Bars);
        assert_eq!(tokens[5].kind, TokenKind::RBracket);
    }

    #[test]
    fn lex_negative_number() {
        let mut lexer = Lexer::new("-3.5");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Number(-3.5));
    }

    #[test]
    fn lex_colon_and_comma() {
        let mut lexer = Lexer::new("kick: [X] , vel");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Ident("kick".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::Colon);
    }

    #[test]
    fn lex_empty_input() {
        let mut lexer = Lexer::new("");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }

    #[test]
    fn lex_string_literal() {
        let mut lexer = Lexer::new(r#""hello""#);
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Ident("hello".to_string()));
    }
}
