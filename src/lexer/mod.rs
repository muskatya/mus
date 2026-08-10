pub mod tokens;

use crate::errors::{Error, Span};
use tokens::{TokenKind, Token};

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    pub tokens: Vec<Token>,
}

impl Lexer {
    pub fn new(source: String) -> Lexer {
        Lexer { chars: source.chars().collect(), pos: 0, line: 1, col: 1, tokens: Vec::new() }
    }

    fn span(&self) -> Span {
        Span::new(self.line, self.col)
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
        self.col += 1;
    }

    pub fn tokenize(&mut self) -> Result<(), Error> {
        while !self.is_eof() {
            self.skip_whitespaces();
            if self.is_eof() { break; }
            let c = self.chars[self.pos];
            match c {
                '+' => self.tokens.push(Token::new(TokenKind::Plus, self.span())),
                '-' => self.tokens.push(Token::new(TokenKind::Minus, self.span())),
                '*' => self.tokens.push(Token::new(TokenKind::Asterisk, self.span())),
                '(' => self.tokens.push(Token::new(TokenKind::LParen, self.span())),
                ')' => self.tokens.push(Token::new(TokenKind::RParen, self.span())),
                '[' => self.tokens.push(Token::new(TokenKind::LSquare, self.span())),
                ']' => self.tokens.push(Token::new(TokenKind::RSquare, self.span())),
                '{' => self.tokens.push(Token::new(TokenKind::LBrace, self.span())),
                '}' => self.tokens.push(Token::new(TokenKind::RBrace, self.span())),
                ',' => self.tokens.push(Token::new(TokenKind::Comma, self.span())),
                '.' =>{
                    match (self.peek(), self.chars.get(self.pos + 2)) {
                        (Some('.'), Some('.')) => {
                            self.tokens.push(Token::new(TokenKind::Ellipsis, self.span()));
                            self.advance();
                            self.advance();
                        },
                        _ => self.tokens.push(Token::new(TokenKind::Dot, self.span()))
                    }
                },
                ';' => self.tokens.push(Token::new(TokenKind::Semicolon, self.span())),
                '/' => {
                    match self.peek() {
                        Some('/') => { self.skip_oneline_comment(); continue; },
                        Some('*') => { self.skip_multiline_comment()?; continue; },
                        _ => self.tokens.push(Token::new(TokenKind::Slash, self.span()))
                    }
                },
                '=' => {
                    match self.peek() {
                        Some('=') => {
                            self.tokens.push(Token::new(TokenKind::Eq, self.span()));
                            self.advance();
                        },
                        _ => self.tokens.push(Token::new(TokenKind::Assign, self.span()))
                    }
                },
                '>' => {
                    match self.peek() {
                        Some('=') => {
                            self.tokens.push(Token::new(TokenKind::GreaterEq, self.span()));
                            self.advance();
                        },
                        _ => self.tokens.push(Token::new(TokenKind::Greater, self.span()))
                    }
                },
                '<' => {
                    match self.peek() {
                        Some('=') => {
                            self.tokens.push(Token::new(TokenKind::LowerEq, self.span()));
                            self.advance();
                        },
                        _ => self.tokens.push(Token::new(TokenKind::Lower, self.span()))
                    }
                },
                '!' => {
                    match self.peek() {
                        Some('=') => {
                            self.tokens.push(Token::new(TokenKind::NotEq, self.span()));
                            self.advance();
                        },
                        _ => self.tokens.push(Token::new(TokenKind::Exclamation, self.span()))
                    }
                },
                '|' if self.peek() == Some('|') => {
                    self.tokens.push(Token::new(TokenKind::Or, self.span()));
                    self.advance();
                },
                '&' if self.peek() == Some('&') => {
                    self.tokens.push(Token::new(TokenKind::And, self.span()));
                    self.advance();
                },
                '0'..='9' => {
                    let token = self.lex_number()?;
                    self.tokens.push(token);
                    continue;
                },
                '\'' => {
                    let span = self.span();
                    self.advance();
                    if self.is_eof() {
                        return Err(Error::UnclosedChar { span });
                    }
                    if self.chars[self.pos] == '\'' {
                        return Err(Error::EmptyChar { span });
                    }
                    let ch = match self.chars[self.pos] {
                        '\\' => self.lex_escape()?,
                        c => { self.advance(); c }
                    };
                    if self.is_eof() || self.chars[self.pos] != '\'' {
                        return Err(Error::UnclosedChar { span });
                    }
                    self.tokens.push(Token::new(TokenKind::CharLit(ch), span));
                },
                '"' => {
                    let token = self.lex_string()?;
                    self.tokens.push(token);
                    continue;
                },
                'a'..='z' | 'A'..='Z' | '_' => {
                    let token = self.lex_identifier();
                    self.tokens.push(token);
                    continue;
                },
                _ => return Err(Error::IllegalChar { ch: c, span: self.span() }),
            }
            self.advance();
        }
        self.tokens.push(Token::new(TokenKind::EOF, self.span()));
        Ok(())
    }

    fn skip_whitespaces(&mut self) {
        while !self.is_eof() {
            let c = self.chars[self.pos];
            if !c.is_whitespace() { break; }
            if c == '\n' {
                if let Some(t) = self.tokens.last() && !matches!(t.kind,
                    TokenKind::Semicolon | TokenKind::Comma | TokenKind::Dot |
                    TokenKind::LParen | TokenKind::LSquare | TokenKind::LBrace | TokenKind::RBrace
                ) { self.tokens.push(Token::new(TokenKind::Semicolon, self.span())); }
                self.pos += 1;
                self.line += 1;
                self.col = 1;
                continue;
            }
            self.advance();
        }
    }

    fn skip_oneline_comment(&mut self) {
        while !self.is_eof() {
            if self.chars[self.pos] == '\n' {
                self.advance();
                break;
            }
            self.advance();
        }
    }

    fn skip_multiline_comment(&mut self) -> Result<(), Error> {
        let span = self.span();
        while !self.is_eof() {
            if self.chars[self.pos] == '*' && let Some(n) = self.peek() && n == '/' {
                self.advance();
                self.advance();
                return Ok(());
            }
            self.advance();
        }
        Err(Error::UnterminatedComment { span })
    }

    fn lex_number(&mut self) -> Result<Token, Error> {
        let span = self.span();
        let mut string = String::new();
        let mut points = 0;
        while !self.is_eof() {
            let c = self.chars[self.pos];
            match c {
                '_' => { self.advance(); continue; },
                '.' => points += 1,
                '0'..='9' => {},
                _ => break
            }
            string.push(c);
            self.advance();
        }
        match points {
            0 => Ok(Token::new(TokenKind::Integer(string.parse().unwrap()), span)),
            1 => Ok(Token::new(TokenKind::Float(string.parse().unwrap()), span)),
            _ => Err(Error::TooManyPoints { span })
        }
    }

    fn lex_escape(&mut self) -> Result<char, Error> {
        let span = self.span();
        self.advance();
        let res = match self.chars[self.pos] {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '0' => '\0',
            '\\' => '\\',
            '"' => '"',
            '\'' => '\'',
            e => return Err(Error::InvalidEscape { escape: format!("\\{}", e), span })
        };
        self.advance();
        Ok(res)
    }

    fn lex_string(&mut self) -> Result<Token, Error> {
        let span = self.span();
        let mut string = String::new();
        self.advance();
        while !self.is_eof() {
            let c = self.chars[self.pos];
            match c {
                '\\' => {
                    if self.peek().is_none() { break; }
                    string.push(self.lex_escape()?);
                    continue;
                }
                '"' => {
                    self.advance();
                    return Ok(Token::new(TokenKind::String(string), span));
                },
                _ => string.push(c)
            }
            self.advance();
        }
        Err(Error::UnterminatedString { span })
    }

    fn lex_identifier(&mut self) -> Token {
        let span = self.span();
        let mut string = String::new();
        while !self.is_eof() {
            let c = self.chars[self.pos];
            if !matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_') { break; }
            string.push(c);
            self.advance();
        }
        let kind = match string.as_str() {
            "i64" => TokenKind::I64,
            "i32" => TokenKind::I32,
            "i16" => TokenKind::I16,
            "i8" => TokenKind::I8,
            "u64" => TokenKind::U64,
            "u32" => TokenKind::U32,
            "u16" => TokenKind::U16,
            "u8" => TokenKind::U8,
            "f64" => TokenKind::F64,
            "f32" => TokenKind::F32,
            "bool" => TokenKind::Bool,
            "char" => TokenKind::Char,
            "str" => TokenKind::Str,
            "void" => TokenKind::Void,
            "fn" => TokenKind::Fn,
            "import" => TokenKind::Import,
            "extern" => TokenKind::Extern,
            "var" => TokenKind::Var,
            "const" => TokenKind::Const,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "return" => TokenKind::Return,
            "as" => TokenKind::As,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "continue" => TokenKind::Continue,
            "break" => TokenKind::Break,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            _ => TokenKind::Identifier(string)
        };
        Token::new(kind, span)
    }
}
