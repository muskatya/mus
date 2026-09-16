use crate::frontend::{Token, TokenKind, Error, ErrorKind, Span};

#[derive(Debug)]
pub struct Lexer {
    chars: Vec<char>,
    len: usize,
    pos: usize,
    pub tokens: Vec<Token>,
    file: String,
    source: String
}

impl Lexer {
    pub fn new(source: String, file: String) -> Self {
        let chars: Vec<char> = source.chars().collect();
        let len = chars.len();
        Self { chars, len, pos: 0, tokens: Vec::new(), file, source }
    }

    fn error(&self, span: Span, message: &str, note: Option<&str>) -> Error {
        Error::new(
            ErrorKind::Lexical,
            &self.file,
            &self.source,
            span,
            message,
            note
        )
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.len
    }

    fn can_insert_semicolon(&self) -> bool {
        self.tokens.last().map_or(false, |t| !matches!(
            t.kind, 
            TokenKind::Semicolon | TokenKind::Comma | TokenKind::Dot | TokenKind::Ellipsis |
            TokenKind::LParen | TokenKind::LSquare | TokenKind::LBrace |
            TokenKind::Plus | TokenKind::Minus | TokenKind::Asterisk | TokenKind::Slash |
            TokenKind::Assign |
            TokenKind::Eq | TokenKind::NotEq |
            TokenKind::Greater | TokenKind::Lower | 
            TokenKind::GreaterEq | TokenKind::LowerEq |
            TokenKind::And | TokenKind::Or |
            TokenKind::Exclamation
        ))
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    pub fn tokenize(&mut self) -> Result<(), Error> {
        while !self.is_eof() {
            self.skip_whitespaces();
            let Some(c) = self.chars.get(self.pos).copied() else {
                break;
            };
            let start = self.pos;
            let kind = match c {
                '+' => TokenKind::Plus,
                '-' => TokenKind::Minus,
                '*' => TokenKind::Asterisk,
                '/' => match self.peek() {
                    Some('/') => {
                        self.skip_oneline();
                        continue;
                    },
                    Some('*') => {
                        self.skip_multiline()?;
                        continue;
                    },
                    _ => TokenKind::Slash
                },
                '=' => match self.peek() {
                    Some('=') => {
                        self.pos += 1;
                        TokenKind::Eq
                    },
                    _ => TokenKind::Assign
                },
                '(' => TokenKind::LParen,
                ')' => TokenKind::RParen,
                '[' => TokenKind::LSquare,
                ']' => TokenKind::RSquare,
                '{' => TokenKind::LBrace,
                '}' => TokenKind::RBrace,
                '!' => match self.peek() {
                    Some('=') => {
                        self.pos += 1;
                        TokenKind::NotEq
                    },
                    _ => TokenKind::Exclamation
                },
                '|' if self.peek() == Some('|') => {
                    self.pos += 1;
                    TokenKind::Or
                },
                '&' if self.peek() == Some('&') => {
                    self.pos += 1;
                    TokenKind::And
                },
                ',' => TokenKind::Comma,
                '.' => if self.peek() == Some('.') && self.chars.get(self.pos + 2) == Some(&'.') {
                    self.pos += 2;
                    TokenKind::Ellipsis
                } else {
                    TokenKind::Dot
                },
                ';' => TokenKind::Semicolon,
                '>' => match self.peek() {
                    Some('=') => {
                        self.pos += 1;
                        TokenKind::GreaterEq
                    },
                    _ => TokenKind::Greater
                },
                '<' => match self.peek() {
                    Some('=') => {
                        self.pos += 1;
                        TokenKind::LowerEq
                    },
                    _ => TokenKind::Lower
                },
                '0'..='9' => {
                    let num = self.lex_number()?;
                    self.tokens.push(num);
                    continue;
                },
                'a'..='z' | 'A'..='Z' | '_' => {
                    let ident = self.lex_ident()?;
                    self.tokens.push(ident);
                    continue;
                },
                '\'' => {
                    let c = self.lex_char()?;
                    self.tokens.push(c);
                    continue;
                },
                '"' => {
                    let string = self.lex_string()?;
                    self.tokens.push(string);
                    continue;
                }
                _ => return Err(self.error(
                    Span::short(self.pos),
                    &format!("Illegal character '{}'", c),
                    None
                ))
            };
            let end = self.pos;
            self.pos += 1;
            self.tokens.push(Token::new(kind, Span::new(start, end)));
        }
        if self.can_insert_semicolon() {
            self.tokens.push(Token::new(TokenKind::Semicolon, Span::short(self.pos)));
            self.pos += 1;
        }
        self.tokens.push(Token::new(TokenKind::EOF, Span::short(self.pos)));
        Ok(())
    }

    fn skip_whitespaces(&mut self) {
        while !self.is_eof() {
            let c = self.chars[self.pos];
            if !c.is_whitespace() {
                break;
            }
            if c == '\n' {
                if self.can_insert_semicolon() {
                    self.tokens.push(Token::new(TokenKind::Semicolon, Span::short(self.pos)));
                }
            }
            self.pos += 1;
        }
    }

    fn skip_oneline(&mut self) {
        while !self.is_eof() {
            if self.chars[self.pos] == '\n' {
                break;
            }
            self.pos += 1;
        }
    }

    fn skip_multiline(&mut self) -> Result<(), Error> {
        let start = self.pos;
        self.pos += 2;
        while !self.is_eof() {
            if self.chars[self.pos] == '*' && self.peek() == Some('/') {
                self.pos += 2;
                return Ok(());
            }
            self.pos += 1;
        }
        Err(self.error(
            Span::new(start, self.pos),
            "Unterminated comment",
            None
        ))
    }

    fn lex_number(&mut self) -> Result<Token, Error> {
        let start = self.pos;
        let mut num = String::new();
        let mut points = 0;
        while !self.is_eof() {
            let c = self.chars[self.pos];
            match c {
                '_' => {
                    self.pos += 1;
                    continue;
                },
                '.' => points += 1,
                '0'..='9' => (),
                _ => break
            }
            num.push(c);
            self.pos += 1;
        }
        let span = Span::new(start, self.pos);
        let kind = match points {
            0 => TokenKind::Integer(num.parse().unwrap()),
            1 => TokenKind::Float(num.parse().unwrap()),
            _ => return Err(self.error(
                span,
                &format!("Too many points"),
                None
            ))
        };
        Ok(Token::new(kind, span))
    }

    fn lex_ident(&mut self) -> Result<Token, Error> {
        let start = self.pos;
        let mut ident = String::new();
        while !self.is_eof() {
            let c = self.chars[self.pos];
            if !matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_') {
                break;
            }
            ident.push(c);
            self.pos += 1;
        }
        let kind = match ident.as_str() {
            "i64" => TokenKind::I64, "u64" => TokenKind::U64, "f64" => TokenKind::F64,
            "i32" => TokenKind::I32, "u32" => TokenKind::U32, "f32" => TokenKind::F32,
            "i16" => TokenKind::I16, "u16" => TokenKind::U16,
            "i8" => TokenKind::I8, "u8" => TokenKind::U8,
            "bool" => TokenKind::Bool,
            "char" => TokenKind::Char,
            "str" => TokenKind::Str,
            "void" => TokenKind::Void,
            "pub" => TokenKind::Pub,
            "fn" => TokenKind::Fn,
            "import" => TokenKind::Import, "extern" => TokenKind::Extern,
            "var" => TokenKind::Var, "const" => TokenKind::Const,
            "return" => TokenKind::Return,
            "if" => TokenKind::If, "else" => TokenKind::Else,
            "as" => TokenKind::As,
            "while" => TokenKind::While,
            "for" => TokenKind::For, "in" => TokenKind::In,
            "continue" => TokenKind::Continue, "break" => TokenKind::Break,
            "true" => TokenKind::True, "false" => TokenKind::False,
            _ => TokenKind::Identifier(ident)
        };
        Ok(Token::new(kind, Span::new(start, self.pos)))
    }

    fn lex_escape(&mut self) -> Result<char, Error> {
        let start = self.pos;
        self.pos += 1;
        let c = self.chars.get(self.pos).copied()
            .ok_or_else(|| self.error(
                Span::new(start, self.pos),
                "Unterminated character",
                None
            ))?;
        self.pos += 1;
        let esc = match c {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '\\' => '\\',
            '0' => '\0',
            '\'' => '\'',
            '"' => '"',
            _ => return Err(self.error(
                Span::new(start, self.pos),
                &format!("Unknown character escape '{}'", c),
                Some("Available character escapes are n, r, t, \\, 0, ' and \"")
            ))
        };
        Ok(esc)
    }

    fn lex_char(&mut self) -> Result<Token, Error> {
        let start = self.pos;
        self.pos += 1;
        let c = match self.chars.get(self.pos).copied() {
            Some('\\') => self.lex_escape()?,
            Some('\'') => return Err(self.error(
                Span::new(start, self.pos),
                "Empty character",
                None
            )),
            Some(c) => {
                self.pos += 1;
                c
            },
            None => return Err(self.error(
                Span::new(start, self.pos),
                "Unterminated character",
                None
            ))
        };
        if self.chars.get(self.pos) != Some(&'\'') {
            return Err(self.error(
                Span::new(start, self.pos),
                "Unterminated character",
                None
            ));
        }
        self.pos += 1;
        Ok(Token::new(TokenKind::Character(c), Span::new(start, self.pos)))
    }

    fn lex_string(&mut self) -> Result<Token, Error> {
        let start = self.pos;
        self.pos += 1;
        let mut string = String::new();
        while !self.is_eof() {
            let c = self.chars[self.pos];
            let c = match c {
                '\\' => self.lex_escape()?,
                '"' => {
                    self.pos += 1;
                    return Ok(Token::new(TokenKind::String(string), Span::new(start, self.pos)));
                },
                _ => {
                    self.pos += 1;
                    c
                },
            };
            string.push(c);
        }
        Err(self.error(
            Span::new(start, self.pos),
            "Unterminated string",
            None
        ))
    }
}
