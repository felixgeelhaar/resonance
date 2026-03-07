//! Parser for the Resonance DSL.
//!
//! Parses a token stream into the AST. Supports both declarative and
//! functional chain syntaxes — both produce the same AST types.

use super::ast::*;
use super::error::CompileError;
use super::token::{NoteToken, StepToken, Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<Program, CompileError> {
        let mut tempo = 120.0;
        let mut tracks = Vec::new();
        let mut macros = Vec::new();
        let mut mappings = Vec::new();
        let mut layers = Vec::new();

        self.skip_newlines();

        while !self.is_at_end() {
            self.skip_newlines();
            if self.is_at_end() {
                break;
            }

            match &self.peek().kind {
                TokenKind::Tempo => {
                    tempo = self.parse_tempo()?;
                }
                TokenKind::Track => {
                    tracks.push(self.parse_track()?);
                }
                TokenKind::Macro => {
                    macros.push(self.parse_macro()?);
                }
                TokenKind::Map => {
                    mappings.push(self.parse_mapping()?);
                }
                TokenKind::Layer => {
                    layers.push(self.parse_layer()?);
                }
                TokenKind::Ident(_) => {
                    // Functional chain syntax: name = ...
                    let track = self.parse_functional_track()?;
                    tracks.push(track);
                }
                TokenKind::Eof => break,
                _ => {
                    let t = self.peek();
                    return Err(CompileError::parse(
                        format!("unexpected token: {:?}", t.kind),
                        t.line,
                        t.col,
                    ));
                }
            }
        }

        Ok(Program {
            tempo,
            tracks,
            macros,
            mappings,
            layers,
        })
    }

    fn parse_tempo(&mut self) -> Result<f64, CompileError> {
        self.expect(TokenKind::Tempo)?;
        let val = self.expect_number()?;
        self.skip_newlines();
        Ok(val)
    }

    fn parse_track(&mut self) -> Result<TrackDef, CompileError> {
        self.expect(TokenKind::Track)?;
        let name = self.expect_name()?;
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();

        let instrument = self.parse_instrument_ref()?;
        self.skip_newlines();

        let mut sections = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            self.skip_newlines();
            if self.check(TokenKind::RBrace) {
                break;
            }
            sections.push(self.parse_section()?);
            self.skip_newlines();
        }
        self.expect(TokenKind::RBrace)?;

        Ok(TrackDef {
            name,
            instrument,
            sections,
        })
    }

    fn parse_instrument_ref(&mut self) -> Result<InstrumentRef, CompileError> {
        let t = self.peek();
        match &t.kind {
            TokenKind::Kit => {
                self.advance();
                self.expect(TokenKind::Colon)?;
                let name = self.expect_ident()?;
                Ok(InstrumentRef::Kit(name))
            }
            TokenKind::Bass => {
                self.advance();
                Ok(InstrumentRef::Bass)
            }
            TokenKind::Poly => {
                self.advance();
                Ok(InstrumentRef::Poly)
            }
            TokenKind::Pluck => {
                self.advance();
                Ok(InstrumentRef::Pluck)
            }
            TokenKind::Noise => {
                self.advance();
                Ok(InstrumentRef::Noise)
            }
            TokenKind::Plugin => {
                self.advance();
                self.expect(TokenKind::Colon)?;
                let name = self.expect_ident()?;
                Ok(InstrumentRef::Plugin(name))
            }
            TokenKind::Ident(s) if s == "kit" => {
                self.advance();
                self.expect(TokenKind::Colon)?;
                let name = self.expect_ident()?;
                Ok(InstrumentRef::Kit(name))
            }
            _ => Err(CompileError::parse(
                format!("expected instrument type, got {:?}", t.kind),
                t.line,
                t.col,
            )),
        }
    }

    fn parse_section(&mut self) -> Result<SectionDef, CompileError> {
        self.expect(TokenKind::Section)?;
        let name = self.expect_ident()?;

        // Parse [N bars]
        self.expect(TokenKind::LBracket)?;
        let length_bars = self.expect_integer()? as u32;
        self.expect(TokenKind::Bars)?;
        self.expect(TokenKind::RBracket)?;

        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();

        let mut patterns = Vec::new();
        let mut overrides = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            self.skip_newlines();
            if self.check(TokenKind::RBrace) {
                break;
            }
            if self.check(TokenKind::Override) {
                overrides.push(self.parse_override()?);
            } else {
                patterns.push(self.parse_pattern()?);
            }
            self.skip_newlines();
        }
        self.expect(TokenKind::RBrace)?;

        Ok(SectionDef {
            name,
            length_bars,
            patterns,
            overrides,
        })
    }

    /// Parse an override line: `override macro_name -> target_param (lo..hi) curve`
    fn parse_override(&mut self) -> Result<MappingOverrideDef, CompileError> {
        self.expect(TokenKind::Override)?;
        let macro_name = self.expect_ident()?;
        self.expect(TokenKind::Arrow)?;
        let target_param = self.expect_ident()?;

        let range = if self.check(TokenKind::LParen) {
            self.advance();
            let lo = self.expect_number()?;
            self.expect(TokenKind::DotDot)?;
            let hi = self.expect_number()?;
            self.expect(TokenKind::RParen)?;
            (lo, hi)
        } else {
            (0.0, 1.0)
        };

        let curve = if self.check_ident("linear") {
            self.advance();
            CurveKind::Linear
        } else if self.check_ident("log") {
            self.advance();
            CurveKind::Log
        } else if self.check_ident("exp") {
            self.advance();
            CurveKind::Exp
        } else if self.check_ident("smoothstep") {
            self.advance();
            CurveKind::Smoothstep
        } else {
            CurveKind::Linear
        };

        Ok(MappingOverrideDef {
            macro_name,
            target_param,
            range,
            curve,
        })
    }

    /// Parse a top-level layer block:
    /// ```text
    /// layer reverb_wash {
    ///     depth -> reverb_mix (0.0..0.8) smoothstep
    ///     depth -> delay_mix (0.0..0.4) linear
    /// }
    /// ```
    fn parse_layer(&mut self) -> Result<LayerDef, CompileError> {
        self.expect(TokenKind::Layer)?;
        let name = self.expect_ident()?;
        self.skip_newlines();
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();

        let mut mappings = Vec::new();

        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            self.skip_newlines();
            if self.check(TokenKind::RBrace) {
                break;
            }

            // Each line: macro_name -> target_param (lo..hi) curve
            let macro_name = self.expect_ident()?;
            self.expect(TokenKind::Arrow)?;
            let target_param = self.expect_ident()?;

            let range = if self.check(TokenKind::LParen) {
                self.advance();
                let lo = self.expect_number()?;
                self.expect(TokenKind::DotDot)?;
                let hi = self.expect_number()?;
                self.expect(TokenKind::RParen)?;
                (lo, hi)
            } else {
                (0.0, 1.0)
            };

            let curve = if self.check_ident("linear") {
                self.advance();
                CurveKind::Linear
            } else if self.check_ident("log") {
                self.advance();
                CurveKind::Log
            } else if self.check_ident("exp") {
                self.advance();
                CurveKind::Exp
            } else if self.check_ident("smoothstep") {
                self.advance();
                CurveKind::Smoothstep
            } else {
                CurveKind::Linear
            };

            mappings.push(MappingDef {
                macro_name,
                target_param,
                range,
                curve,
            });
            self.skip_newlines();
        }

        self.expect(TokenKind::RBrace)?;
        self.skip_newlines();

        Ok(LayerDef {
            name,
            mappings,
            enabled_by_default: false,
        })
    }

    fn parse_pattern(&mut self) -> Result<PatternDef, CompileError> {
        let target = self.expect_ident()?;
        self.expect(TokenKind::Colon)?;

        let steps = self.parse_steps()?;

        // Optional velocity array
        let velocities = if self.check_skip_newlines(TokenKind::Vel) {
            self.advance(); // consume 'vel'
            Some(self.parse_velocity_array()?)
        } else {
            None
        };

        Ok(PatternDef {
            target,
            steps,
            velocities,
            transforms: vec![],
        })
    }

    fn parse_steps(&mut self) -> Result<Vec<Step>, CompileError> {
        let t = self.peek();
        match &t.kind {
            TokenKind::StepPattern(steps) => {
                let result: Vec<Step> = steps
                    .iter()
                    .map(|s| match s {
                        StepToken::Hit | StepToken::Accent => Step::Hit,
                        StepToken::Ghost => Step::Accent(0.5),
                        StepToken::Rest => Step::Rest,
                    })
                    .collect();
                self.advance();
                Ok(result)
            }
            TokenKind::NotePattern(notes) => {
                let result: Vec<Step> = notes
                    .iter()
                    .map(|n| match n {
                        NoteToken::Note(name) => Step::Note(name.clone()),
                        NoteToken::Rest => Step::Rest,
                    })
                    .collect();
                self.advance();
                Ok(result)
            }
            _ => Err(CompileError::parse(
                format!("expected pattern, got {:?}", t.kind),
                t.line,
                t.col,
            )),
        }
    }

    fn parse_velocity_array(&mut self) -> Result<Vec<f64>, CompileError> {
        let t = self.peek();
        match &t.kind {
            TokenKind::StepPattern(steps) => {
                // StepPattern from numeric arrays — rest=0, hit=non-zero
                // We need to re-interpret the original bracket content
                let result: Vec<f64> = steps
                    .iter()
                    .map(|s| match s {
                        StepToken::Rest => 0.0,
                        StepToken::Hit | StepToken::Accent => 1.0,
                        StepToken::Ghost => 0.5,
                    })
                    .collect();
                self.advance();
                Ok(result)
            }
            _ => Err(CompileError::parse(
                format!("expected velocity array, got {:?}", t.kind),
                t.line,
                t.col,
            )),
        }
    }

    /// Parse a functional chain: `name = instrument(...) |> ...`
    fn parse_functional_track(&mut self) -> Result<TrackDef, CompileError> {
        let name = self.expect_ident()?;
        self.expect(TokenKind::Eq)?;

        let instrument = self.parse_functional_instrument()?;

        let mut patterns = Vec::new();

        // Parse chain: |> target.method(args)
        while self.check(TokenKind::Pipe) || self.check_skip_newlines(TokenKind::Pipe) {
            self.advance(); // consume |>
            let pattern = self.parse_chain_step()?;
            patterns.push(pattern);
        }

        // Wrap all patterns in a default section
        let sections = if patterns.is_empty() {
            Vec::new()
        } else {
            vec![SectionDef {
                name: "main".to_string(),
                length_bars: 2,
                patterns,
                overrides: vec![],
            }]
        };

        Ok(TrackDef {
            name,
            instrument,
            sections,
        })
    }

    fn parse_functional_instrument(&mut self) -> Result<InstrumentRef, CompileError> {
        let t = self.peek();
        match &t.kind {
            TokenKind::Kit => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let _name = self.expect_string_literal()?;
                self.expect(TokenKind::RParen)?;
                Ok(InstrumentRef::Kit(_name))
            }
            TokenKind::Plugin => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let name = self.expect_string_literal()?;
                self.expect(TokenKind::RParen)?;
                Ok(InstrumentRef::Plugin(name))
            }
            TokenKind::Bass => {
                self.advance();
                if self.check(TokenKind::LParen) {
                    self.advance();
                    self.expect(TokenKind::RParen)?;
                }
                Ok(InstrumentRef::Bass)
            }
            TokenKind::Poly => {
                self.advance();
                if self.check(TokenKind::LParen) {
                    self.advance();
                    self.expect(TokenKind::RParen)?;
                }
                Ok(InstrumentRef::Poly)
            }
            TokenKind::Pluck => {
                self.advance();
                if self.check(TokenKind::LParen) {
                    self.advance();
                    self.expect(TokenKind::RParen)?;
                }
                Ok(InstrumentRef::Pluck)
            }
            TokenKind::Noise => {
                self.advance();
                if self.check(TokenKind::LParen) {
                    self.advance();
                    self.expect(TokenKind::RParen)?;
                }
                Ok(InstrumentRef::Noise)
            }
            _ => Err(CompileError::parse(
                format!("expected instrument, got {:?}", t.kind),
                t.line,
                t.col,
            )),
        }
    }

    fn parse_chain_step(&mut self) -> Result<PatternDef, CompileError> {
        let target = self.expect_ident()?;

        self.expect(TokenKind::Dot)?;
        let method = self.expect_ident()?;

        self.expect(TokenKind::LParen)?;

        let steps = match method.as_str() {
            "pattern" => {
                let pattern_str = self.expect_string_literal()?;
                self.parse_inline_pattern(&pattern_str)?
            }
            "at" => {
                // at([1, 3]) — beat positions
                let positions = self.parse_number_list()?;
                positions_to_steps(&positions)
            }
            "every" => {
                // every(1/8) — regular interval
                let interval = self.expect_number()?;
                interval_to_steps(interval)
            }
            _ => {
                return Err(CompileError::parse(
                    format!("unknown chain method: {method}"),
                    self.peek().line,
                    self.peek().col,
                ));
            }
        };

        self.expect(TokenKind::RParen)?;

        // Optional .vel(...)
        let velocities = if self.check(TokenKind::Dot) {
            let saved = self.pos;
            self.advance();
            if self.check_ident("vel") || self.check(TokenKind::Vel) {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let vels = self.parse_velocity_arg()?;
                self.expect(TokenKind::RParen)?;
                Some(vels)
            } else {
                self.pos = saved;
                None
            }
        } else {
            None
        };

        // Parse optional transform chain: .fast(2).rev()
        let transforms = self.parse_transform_chain()?;

        Ok(PatternDef {
            target,
            steps,
            velocities,
            transforms,
        })
    }

    /// Parse a chain of transforms: `.fast(2).rev().degrade(0.3)`
    fn parse_transform_chain(&mut self) -> Result<Vec<Transform>, CompileError> {
        let mut transforms = Vec::new();

        while self.check(TokenKind::Dot) {
            let saved = self.pos;
            self.advance(); // consume '.'

            let t = self.peek();
            let method = match &t.kind {
                TokenKind::Ident(s) => s.clone(),
                _ => {
                    self.pos = saved;
                    break;
                }
            };

            match method.as_str() {
                "fast" => {
                    self.advance();
                    self.expect(TokenKind::LParen)?;
                    let n = self.expect_number()?;
                    self.expect(TokenKind::RParen)?;
                    transforms.push(Transform::Fast(n));
                }
                "slow" => {
                    self.advance();
                    self.expect(TokenKind::LParen)?;
                    let n = self.expect_number()?;
                    self.expect(TokenKind::RParen)?;
                    transforms.push(Transform::Slow(n));
                }
                "rev" => {
                    self.advance();
                    self.expect(TokenKind::LParen)?;
                    self.expect(TokenKind::RParen)?;
                    transforms.push(Transform::Rev);
                }
                "rotate" => {
                    self.advance();
                    self.expect(TokenKind::LParen)?;
                    let n = self.expect_number()?;
                    self.expect(TokenKind::RParen)?;
                    transforms.push(Transform::Rotate(n as i32));
                }
                "degrade" => {
                    self.advance();
                    self.expect(TokenKind::LParen)?;
                    let n = self.expect_number()?;
                    self.expect(TokenKind::RParen)?;
                    transforms.push(Transform::Degrade(n));
                }
                "every" => {
                    self.advance();
                    self.expect(TokenKind::LParen)?;
                    let n = self.expect_number()? as u32;
                    self.expect(TokenKind::Comma)?;
                    let inner = self.parse_inner_transform()?;
                    self.expect(TokenKind::RParen)?;
                    transforms.push(Transform::Every(n, Box::new(inner)));
                }
                "sometimes" => {
                    self.advance();
                    self.expect(TokenKind::LParen)?;
                    let prob = self.expect_number()?;
                    self.expect(TokenKind::Comma)?;
                    let inner = self.parse_inner_transform()?;
                    self.expect(TokenKind::RParen)?;
                    transforms.push(Transform::Sometimes(prob, Box::new(inner)));
                }
                "chop" => {
                    self.advance();
                    self.expect(TokenKind::LParen)?;
                    let n = self.expect_number()? as u32;
                    self.expect(TokenKind::RParen)?;
                    transforms.push(Transform::Chop(n));
                }
                "stutter" => {
                    self.advance();
                    self.expect(TokenKind::LParen)?;
                    let n = self.expect_number()? as u32;
                    self.expect(TokenKind::RParen)?;
                    transforms.push(Transform::Stutter(n));
                }
                "add" => {
                    self.advance();
                    self.expect(TokenKind::LParen)?;
                    let n = self.expect_number()? as i32;
                    self.expect(TokenKind::RParen)?;
                    transforms.push(Transform::Add(n));
                }
                "gain" => {
                    self.advance();
                    self.expect(TokenKind::LParen)?;
                    let n = self.expect_number()?;
                    self.expect(TokenKind::RParen)?;
                    transforms.push(Transform::Gain(n));
                }
                "legato" => {
                    self.advance();
                    self.expect(TokenKind::LParen)?;
                    let n = self.expect_number()?;
                    self.expect(TokenKind::RParen)?;
                    transforms.push(Transform::Legato(n));
                }
                _ => {
                    // Not a known transform — backtrack
                    self.pos = saved;
                    break;
                }
            }
        }

        Ok(transforms)
    }

    /// Parse a transform name inside `every(N, ...)` or `sometimes(prob, ...)`.
    fn parse_inner_transform(&mut self) -> Result<Transform, CompileError> {
        let name = self.expect_ident()?;
        match name.as_str() {
            "rev" => Ok(Transform::Rev),
            "fast" => {
                self.expect(TokenKind::LParen)?;
                let n = self.expect_number()?;
                self.expect(TokenKind::RParen)?;
                Ok(Transform::Fast(n))
            }
            "slow" => {
                self.expect(TokenKind::LParen)?;
                let n = self.expect_number()?;
                self.expect(TokenKind::RParen)?;
                Ok(Transform::Slow(n))
            }
            "rotate" => {
                self.expect(TokenKind::LParen)?;
                let n = self.expect_number()? as i32;
                self.expect(TokenKind::RParen)?;
                Ok(Transform::Rotate(n))
            }
            "degrade" => {
                self.expect(TokenKind::LParen)?;
                let n = self.expect_number()?;
                self.expect(TokenKind::RParen)?;
                Ok(Transform::Degrade(n))
            }
            "chop" => {
                self.expect(TokenKind::LParen)?;
                let n = self.expect_number()? as u32;
                self.expect(TokenKind::RParen)?;
                Ok(Transform::Chop(n))
            }
            "stutter" => {
                self.expect(TokenKind::LParen)?;
                let n = self.expect_number()? as u32;
                self.expect(TokenKind::RParen)?;
                Ok(Transform::Stutter(n))
            }
            _ => Err(CompileError::parse(
                format!("unknown inner transform: {name}"),
                self.peek().line,
                self.peek().col,
            )),
        }
    }

    fn parse_inline_pattern(&mut self, s: &str) -> Result<Vec<Step>, CompileError> {
        parse_mini_notation(s)
    }

    fn parse_velocity_arg(&mut self) -> Result<Vec<f64>, CompileError> {
        let t = self.peek();
        match &t.kind {
            TokenKind::Number(v) => {
                let val = *v;
                self.advance();
                // Single velocity — applied to all steps
                Ok(vec![val])
            }
            TokenKind::StepPattern(_) => self.parse_velocity_array(),
            _ => self.parse_number_list(),
        }
    }

    fn parse_number_list(&mut self) -> Result<Vec<f64>, CompileError> {
        let t = self.peek();
        match &t.kind {
            TokenKind::StepPattern(_) => {
                // This was a bracketed numeric array
                self.parse_velocity_array()
            }
            _ => {
                // Comma-separated numbers
                let mut numbers = Vec::new();
                numbers.push(self.expect_number()?);
                while self.check(TokenKind::Comma) {
                    self.advance();
                    numbers.push(self.expect_number()?);
                }
                Ok(numbers)
            }
        }
    }

    fn parse_macro(&mut self) -> Result<MacroDef, CompileError> {
        self.expect(TokenKind::Macro)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::Eq)?;
        let default_value = self.expect_number()?;
        Ok(MacroDef {
            name,
            default_value,
        })
    }

    fn parse_mapping(&mut self) -> Result<MappingDef, CompileError> {
        self.expect(TokenKind::Map)?;
        let macro_name = self.expect_ident()?;
        self.expect(TokenKind::Arrow)?;
        let target_param = self.expect_ident()?;

        // Optional range and curve
        let range = if self.check(TokenKind::LParen) {
            self.advance();
            let lo = self.expect_number()?;
            self.expect(TokenKind::DotDot)?;
            let hi = self.expect_number()?;
            self.expect(TokenKind::RParen)?;
            (lo, hi)
        } else {
            (0.0, 1.0)
        };

        let curve = if self.check_ident("linear") {
            self.advance();
            CurveKind::Linear
        } else if self.check_ident("log") {
            self.advance();
            CurveKind::Log
        } else if self.check_ident("exp") {
            self.advance();
            CurveKind::Exp
        } else if self.check_ident("smoothstep") {
            self.advance();
            CurveKind::Smoothstep
        } else {
            CurveKind::Linear
        };

        Ok(MappingDef {
            macro_name,
            target_param,
            range,
            curve,
        })
    }

    fn expect_string_literal(&mut self) -> Result<String, CompileError> {
        let t = self.peek();
        match &t.kind {
            TokenKind::Ident(s) => {
                let val = s.clone();
                self.advance();
                Ok(val)
            }
            _ => Err(CompileError::parse(
                format!("expected string/identifier, got {:?}", t.kind),
                t.line,
                t.col,
            )),
        }
    }

    // --- Utility methods ---

    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos.min(self.tokens.len() - 1)];
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len() || self.peek().kind == TokenKind::Eof
    }

    fn check(&self, kind: TokenKind) -> bool {
        !self.is_at_end()
            && std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(&kind)
    }

    fn check_ident(&self, name: &str) -> bool {
        matches!(&self.peek().kind, TokenKind::Ident(s) if s == name)
    }

    fn check_skip_newlines(&mut self, kind: TokenKind) -> bool {
        let saved = self.pos;
        self.skip_newlines();
        if self.check(kind) {
            true
        } else {
            self.pos = saved;
            false
        }
    }

    fn skip_newlines(&mut self) {
        while !self.is_at_end() && self.peek().kind == TokenKind::Newline {
            self.pos += 1;
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<&Token, CompileError> {
        self.skip_newlines();
        if std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(&kind) {
            Ok(self.advance())
        } else {
            let t = self.peek();
            Err(CompileError::parse(
                format!("expected {kind:?}, got {:?}", t.kind),
                t.line,
                t.col,
            ))
        }
    }

    /// Accept an identifier or keyword as a name (for track/section names
    /// that might collide with keywords like "bass").
    fn expect_name(&mut self) -> Result<String, CompileError> {
        self.skip_newlines();
        let t = self.peek();
        let name = match &t.kind {
            TokenKind::Ident(s) => s.clone(),
            TokenKind::Bass => "bass".to_string(),
            TokenKind::Poly => "poly".to_string(),
            TokenKind::Pluck => "pluck".to_string(),
            TokenKind::Noise => "noise".to_string(),
            TokenKind::Kit => "kit".to_string(),
            TokenKind::Plugin => "plugin".to_string(),
            _ => {
                return Err(CompileError::parse(
                    format!("expected name, got {:?}", t.kind),
                    t.line,
                    t.col,
                ));
            }
        };
        self.advance();
        Ok(name)
    }

    fn expect_ident(&mut self) -> Result<String, CompileError> {
        self.skip_newlines();
        let t = self.peek();
        match &t.kind {
            TokenKind::Ident(s) => {
                let val = s.clone();
                self.advance();
                Ok(val)
            }
            _ => Err(CompileError::parse(
                format!("expected identifier, got {:?}", t.kind),
                t.line,
                t.col,
            )),
        }
    }

    fn expect_number(&mut self) -> Result<f64, CompileError> {
        self.skip_newlines();
        let t = self.peek();
        match &t.kind {
            TokenKind::Number(v) => {
                let val = *v;
                self.advance();
                Ok(val)
            }
            TokenKind::Integer(v) => {
                let val = *v as f64;
                self.advance();
                Ok(val)
            }
            _ => Err(CompileError::parse(
                format!("expected number, got {:?}", t.kind),
                t.line,
                t.col,
            )),
        }
    }

    fn expect_integer(&mut self) -> Result<u64, CompileError> {
        self.skip_newlines();
        let t = self.peek();
        match &t.kind {
            TokenKind::Integer(v) => {
                let val = *v;
                self.advance();
                Ok(val)
            }
            _ => Err(CompileError::parse(
                format!("expected integer, got {:?}", t.kind),
                t.line,
                t.col,
            )),
        }
    }
}

fn positions_to_steps(positions: &[f64]) -> Vec<Step> {
    if positions.is_empty() {
        return Vec::new();
    }
    let max_pos = positions.iter().cloned().fold(0.0f64, f64::max);
    let len = (max_pos as usize + 1).max(8);
    let mut steps = vec![Step::Rest; len];
    for &pos in positions {
        let idx = pos as usize;
        if idx < len {
            steps[idx] = Step::Hit;
        }
    }
    steps
}

fn interval_to_steps(interval: f64) -> Vec<Step> {
    if interval <= 0.0 {
        return Vec::new();
    }
    // Assume 1 bar = 4 beats; generate steps for one bar
    let num_steps = (4.0 / interval).round() as usize;
    let steps = vec![Step::Hit; num_steps.max(1)];
    steps
}

/// Parse a mini-notation pattern string into steps.
///
/// Supports:
/// - `X` = Hit, `x` = Accent(0.5), `.` = Rest
/// - `X!N` — element repeat: repeat that step N times
/// - `[...]*N` — group repeat: repeat inner content N times
/// - `E(hits,steps)` or `E(hits,steps,rotation)` — Euclidean rhythm
/// - `?` or `?0.7` — random hit with probability
/// - `<X x .>` — alternate per cycle
/// - `{X . x}` — subdivide (fit multiple steps into one slot)
/// - `X^3` — ratchet (rapid repeated hits in one step)
fn parse_mini_notation(s: &str) -> Result<Vec<Step>, CompileError> {
    let bytes = s.as_bytes();
    let mut steps = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'[' => {
                // Find the matching ']'
                let start = i + 1;
                let mut depth = 1u32;
                let mut j = start;
                while j < bytes.len() && depth > 0 {
                    match bytes[j] {
                        b'[' => depth += 1,
                        b']' => depth -= 1,
                        _ => {}
                    }
                    if depth > 0 {
                        j += 1;
                    }
                }
                if depth != 0 {
                    return Err(CompileError::parse(
                        "unmatched '[' in pattern".to_string(),
                        0,
                        i,
                    ));
                }
                // j points at the matching ']'
                let inner = &s[start..j];
                let inner_steps = parse_mini_notation(inner)?;
                i = j + 1; // skip past ']'

                // Check for *N suffix
                let repeat = parse_repeat_suffix(bytes, &mut i, b'*');
                for _ in 0..repeat {
                    steps.extend_from_slice(&inner_steps);
                }
            }
            b'E' if i + 1 < bytes.len() && bytes[i + 1] == b'(' => {
                // Euclidean rhythm: E(hits,steps) or E(hits,steps,rotation)
                i += 2; // skip 'E('
                let hits = parse_int_at(bytes, &mut i);
                if i < bytes.len() && bytes[i] == b',' {
                    i += 1;
                }
                let total = parse_int_at(bytes, &mut i);
                let rotation = if i < bytes.len() && bytes[i] == b',' {
                    i += 1;
                    parse_int_at(bytes, &mut i)
                } else {
                    0
                };
                if i < bytes.len() && bytes[i] == b')' {
                    i += 1;
                }
                let euclidean = euclidean_rhythm(hits, total, rotation);
                steps.extend(euclidean);
            }
            b'?' => {
                // Random hit: ? = 0.5, ?0.7 = 0.7
                i += 1;
                let prob = parse_probability(bytes, &mut i);
                let step = Step::Random(prob);
                // Check for ^N ratchet suffix
                if i < bytes.len() && bytes[i] == b'^' {
                    i += 1;
                    let count = parse_int_at(bytes, &mut i).max(1);
                    steps.push(Step::Ratchet(Box::new(step), count as u32));
                } else {
                    // Check for !N element repeat
                    let repeat = parse_repeat_suffix(bytes, &mut i, b'!');
                    for _ in 0..repeat {
                        steps.push(step.clone());
                    }
                }
            }
            b'<' => {
                // Alternation: <X x .>
                let start = i + 1;
                let mut j = start;
                while j < bytes.len() && bytes[j] != b'>' {
                    j += 1;
                }
                if j >= bytes.len() {
                    return Err(CompileError::parse(
                        "unmatched '<' in pattern".to_string(),
                        0,
                        i,
                    ));
                }
                let inner = &s[start..j];
                // Parse each space-separated token as a step
                let alt_steps: Vec<Step> = inner
                    .split_whitespace()
                    .map(|tok| match tok {
                        "X" => Step::Hit,
                        "x" => Step::Accent(0.5),
                        "." => Step::Rest,
                        _ => Step::Rest,
                    })
                    .collect();
                i = j + 1; // skip past '>'
                if alt_steps.is_empty() {
                    steps.push(Step::Rest);
                } else {
                    steps.push(Step::Alternate(alt_steps));
                }
            }
            b'{' => {
                // Subdivision: {X . x}
                let start = i + 1;
                let mut j = start;
                let mut depth = 1u32;
                while j < bytes.len() && depth > 0 {
                    match bytes[j] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        _ => {}
                    }
                    if depth > 0 {
                        j += 1;
                    }
                }
                if depth != 0 {
                    return Err(CompileError::parse(
                        "unmatched '{' in pattern".to_string(),
                        0,
                        i,
                    ));
                }
                let inner = &s[start..j];
                let inner_steps = parse_mini_notation(inner)?;
                i = j + 1; // skip past '}'
                if inner_steps.is_empty() {
                    steps.push(Step::Rest);
                } else {
                    steps.push(Step::Subdivided(inner_steps));
                }
            }
            b'X' | b'x' | b'.' => {
                let step = match bytes[i] {
                    b'X' => Step::Hit,
                    b'x' => Step::Accent(0.5),
                    _ => Step::Rest,
                };
                i += 1;

                // Check for ^N ratchet suffix (before !N)
                if i < bytes.len() && bytes[i] == b'^' {
                    i += 1;
                    let count = parse_int_at(bytes, &mut i).max(1);
                    steps.push(Step::Ratchet(Box::new(step), count as u32));
                } else {
                    // Check for !N suffix
                    let repeat = parse_repeat_suffix(bytes, &mut i, b'!');
                    for _ in 0..repeat {
                        steps.push(step.clone());
                    }
                }
            }
            b' ' => {
                i += 1;
            }
            _ => {
                // Unknown characters treated as rest
                steps.push(Step::Rest);
                i += 1;
            }
        }
    }

    Ok(steps)
}

/// Bjorklund's algorithm — distribute `hits` evenly across `total` steps.
/// Returns a `Vec<Step>` of Hit and Rest, optionally rotated by `rotation`.
fn euclidean_rhythm(hits: usize, total: usize, rotation: usize) -> Vec<Step> {
    if total == 0 {
        return Vec::new();
    }
    let hits = hits.min(total);

    // Bjorklund's algorithm: distribute ones as evenly as possible
    let mut pattern = vec![false; total];
    if hits == 0 {
        return pattern.into_iter().map(|_| Step::Rest).collect();
    }
    if hits == total {
        return vec![Step::Hit; total];
    }

    // Build using the Bresenham-style distribution
    let mut previous = 0;
    for (i, slot) in pattern.iter_mut().enumerate() {
        let current = ((i + 1) * hits) / total;
        *slot = current != previous;
        previous = current;
    }

    // Apply rotation
    if rotation > 0 {
        pattern.rotate_left(rotation % total);
    }

    pattern
        .into_iter()
        .map(|hit| if hit { Step::Hit } else { Step::Rest })
        .collect()
}

/// Parse a probability value after `?`. Returns 0.5 if no digit follows.
fn parse_probability(bytes: &[u8], i: &mut usize) -> f64 {
    if *i < bytes.len() && (bytes[*i] == b'0' || bytes[*i] == b'1') {
        let start = *i;
        // Consume digits and optional decimal point
        while *i < bytes.len() && (bytes[*i].is_ascii_digit() || bytes[*i] == b'.') {
            *i += 1;
        }
        if start < *i {
            if let Ok(val) = std::str::from_utf8(&bytes[start..*i])
                .unwrap_or("0.5")
                .parse::<f64>()
            {
                return val.clamp(0.0, 1.0);
            }
        }
    }
    0.5
}

/// Parse an integer at current position, advancing `i`.
fn parse_int_at(bytes: &[u8], i: &mut usize) -> usize {
    // Skip whitespace
    while *i < bytes.len() && bytes[*i] == b' ' {
        *i += 1;
    }
    let start = *i;
    while *i < bytes.len() && bytes[*i].is_ascii_digit() {
        *i += 1;
    }
    if start < *i {
        std::str::from_utf8(&bytes[start..*i])
            .unwrap_or("0")
            .parse()
            .unwrap_or(0)
    } else {
        0
    }
}

/// Parse a repeat suffix (`*N` or `!N`) starting at position `i`.
/// Returns the repeat count (1 if no suffix found).
fn parse_repeat_suffix(bytes: &[u8], i: &mut usize, marker: u8) -> usize {
    if *i < bytes.len() && bytes[*i] == marker {
        *i += 1; // skip marker
        let start = *i;
        while *i < bytes.len() && bytes[*i].is_ascii_digit() {
            *i += 1;
        }
        if start < *i {
            let n: usize = std::str::from_utf8(&bytes[start..*i])
                .unwrap()
                .parse()
                .unwrap_or(1);
            n.max(1)
        } else {
            1
        }
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::lexer::Lexer;

    fn parse(src: &str) -> Result<Program, CompileError> {
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    #[test]
    fn parse_empty_program() {
        let prog = parse("").unwrap();
        assert_eq!(prog.tempo, 120.0);
        assert!(prog.tracks.is_empty());
    }

    #[test]
    fn parse_tempo() {
        let prog = parse("tempo 128").unwrap();
        assert!((prog.tempo - 128.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_minimal_drum_track() {
        let src = r#"
tempo 128

track drums {
  kit: default
  section groove [2 bars] {
    kick: [X . . x . X . .]
  }
}
"#;
        let prog = parse(src).unwrap();
        assert!((prog.tempo - 128.0).abs() < f64::EPSILON);
        assert_eq!(prog.tracks.len(), 1);

        let track = &prog.tracks[0];
        assert_eq!(track.name, "drums");
        assert_eq!(track.instrument, InstrumentRef::Kit("default".to_string()));
        assert_eq!(track.sections.len(), 1);

        let section = &track.sections[0];
        assert_eq!(section.name, "groove");
        assert_eq!(section.length_bars, 2);
        assert_eq!(section.patterns.len(), 1);
        assert_eq!(section.patterns[0].target, "kick");
        assert_eq!(section.patterns[0].steps.len(), 8);
    }

    #[test]
    fn parse_drum_track_with_velocity() {
        let src = r#"
track drums {
  kit: default
  section main [2 bars] {
    kick: [X . . x . X . .] vel [X . . x . X . .]
    snare: [. X . . . . X .]
  }
}
"#;
        let prog = parse(src).unwrap();
        let track = &prog.tracks[0];
        let section = &track.sections[0];
        assert_eq!(section.patterns.len(), 2);
        assert!(section.patterns[0].velocities.is_some());
        assert!(section.patterns[1].velocities.is_none());
    }

    #[test]
    fn parse_bass_track() {
        let src = r#"
track bass {
  bass
  section groove [2 bars] {
    note: [C2 . . C2 . . Eb2 .]
  }
}
"#;
        let prog = parse(src).unwrap();
        let track = &prog.tracks[0];
        assert_eq!(track.name, "bass");
        assert_eq!(track.instrument, InstrumentRef::Bass);

        let pattern = &track.sections[0].patterns[0];
        assert_eq!(pattern.steps.len(), 8);
        assert_eq!(pattern.steps[0], Step::Note("C2".to_string()));
        assert_eq!(pattern.steps[1], Step::Rest);
        assert_eq!(pattern.steps[6], Step::Note("Eb2".to_string()));
    }

    #[test]
    fn parse_multiple_tracks() {
        let src = r#"
tempo 140

track drums {
  kit: default
  section main [2 bars] {
    kick: [X . . . X . . .]
  }
}

track bass {
  bass
  section main [2 bars] {
    note: [C2 . . C2 . . . .]
  }
}
"#;
        let prog = parse(src).unwrap();
        assert_eq!(prog.tracks.len(), 2);
        assert_eq!(prog.tracks[0].name, "drums");
        assert_eq!(prog.tracks[1].name, "bass");
    }

    #[test]
    fn parse_macro_definition() {
        let src = "macro filter = 0.5";
        let prog = parse(src).unwrap();
        assert_eq!(prog.macros.len(), 1);
        assert_eq!(prog.macros[0].name, "filter");
        assert!((prog.macros[0].default_value - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_mapping() {
        let src = "map filter -> cutoff (0.0..1.0) smoothstep";
        let prog = parse(src).unwrap();
        assert_eq!(prog.mappings.len(), 1);
        assert_eq!(prog.mappings[0].macro_name, "filter");
        assert_eq!(prog.mappings[0].target_param, "cutoff");
        assert_eq!(prog.mappings[0].range, (0.0, 1.0));
        assert_eq!(prog.mappings[0].curve, CurveKind::Smoothstep);
    }

    #[test]
    fn parse_mapping_default_curve() {
        let src = "map volume -> gain";
        let prog = parse(src).unwrap();
        assert_eq!(prog.mappings[0].curve, CurveKind::Linear);
        assert_eq!(prog.mappings[0].range, (0.0, 1.0));
    }

    #[test]
    fn parse_instrument_types() {
        for (keyword, expected) in [
            ("bass", InstrumentRef::Bass),
            ("poly", InstrumentRef::Poly),
            ("pluck", InstrumentRef::Pluck),
            ("noise", InstrumentRef::Noise),
        ] {
            let src =
                format!("track t {{ {keyword}\n  section s [1 bars] {{ note: [C4 . . .] }} }}");
            let prog = parse(&src).unwrap();
            assert_eq!(prog.tracks[0].instrument, expected, "failed for {keyword}");
        }
    }

    #[test]
    fn parse_error_unexpected_token() {
        let result = parse("123");
        assert!(result.is_err());
    }

    #[test]
    fn parse_error_missing_brace() {
        let result = parse("track drums { kit: default");
        assert!(result.is_err());
    }

    #[test]
    fn parse_complete_song() {
        let src = r#"
tempo 128

macro filter = 0.5
map filter -> cutoff (100.0..8000.0) exp

track drums {
  kit: default
  section groove [2 bars] {
    kick:  [X . . x . X . .]
    snare: [. X . . . . X .]
    hat:   [x x x x x x x x]
  }
}

track bass {
  bass
  section groove [2 bars] {
    note: [C2 . . C2 . . Eb2 .]
  }
}
"#;
        let prog = parse(src).unwrap();
        assert!((prog.tempo - 128.0).abs() < f64::EPSILON);
        assert_eq!(prog.tracks.len(), 2);
        assert_eq!(prog.macros.len(), 1);
        assert_eq!(prog.mappings.len(), 1);

        let drums = &prog.tracks[0];
        assert_eq!(drums.sections[0].patterns.len(), 3);

        let bass = &prog.tracks[1];
        assert_eq!(bass.instrument, InstrumentRef::Bass);
    }

    #[test]
    fn parse_with_comments() {
        let src = r#"
// Set the tempo
tempo 128
// Define drums
track drums {
  kit: default  // use default kit
  section main [2 bars] {
    kick: [X . . . X . . .]
  }
}
"#;
        let prog = parse(src).unwrap();
        assert!((prog.tempo - 128.0).abs() < f64::EPSILON);
        assert_eq!(prog.tracks.len(), 1);
    }

    #[test]
    fn parse_section_with_override() {
        let src = r#"
track drums {
  kit: default
  section verse [4 bars] {
    kick: [X . . . X . . .]
    override filter -> cutoff (0.2..0.6) linear
  }
}
"#;
        let prog = parse(src).unwrap();
        let section = &prog.tracks[0].sections[0];
        assert_eq!(section.overrides.len(), 1);
        assert_eq!(section.overrides[0].macro_name, "filter");
        assert_eq!(section.overrides[0].target_param, "cutoff");
        assert_eq!(section.overrides[0].range, (0.2, 0.6));
        assert_eq!(section.overrides[0].curve, CurveKind::Linear);
    }

    #[test]
    fn parse_section_with_multiple_overrides() {
        let src = r#"
track drums {
  kit: default
  section verse [4 bars] {
    kick: [X . . .]
    override filter -> cutoff (0.2..0.6) linear
    override depth -> reverb_mix (0.0..0.8) smoothstep
  }
}
"#;
        let prog = parse(src).unwrap();
        let section = &prog.tracks[0].sections[0];
        assert_eq!(section.overrides.len(), 2);
        assert_eq!(section.overrides[1].macro_name, "depth");
        assert_eq!(section.overrides[1].curve, CurveKind::Smoothstep);
    }

    #[test]
    fn parse_section_override_default_range_and_curve() {
        let src = r#"
track drums {
  kit: default
  section main [2 bars] {
    kick: [X . . .]
    override filter -> cutoff
  }
}
"#;
        let prog = parse(src).unwrap();
        let ovr = &prog.tracks[0].sections[0].overrides[0];
        assert_eq!(ovr.range, (0.0, 1.0));
        assert_eq!(ovr.curve, CurveKind::Linear);
    }

    #[test]
    fn parse_section_no_overrides_backward_compat() {
        let src = r#"
track drums {
  kit: default
  section main [2 bars] {
    kick: [X . . .]
  }
}
"#;
        let prog = parse(src).unwrap();
        assert!(prog.tracks[0].sections[0].overrides.is_empty());
    }

    #[test]
    fn parse_override_with_exp_curve() {
        let src = r#"
track drums {
  kit: default
  section chorus [4 bars] {
    kick: [X . . .]
    override intensity -> drive (0.0..10.0) exp
  }
}
"#;
        let prog = parse(src).unwrap();
        let ovr = &prog.tracks[0].sections[0].overrides[0];
        assert_eq!(ovr.macro_name, "intensity");
        assert_eq!(ovr.target_param, "drive");
        assert_eq!(ovr.range, (0.0, 10.0));
        assert_eq!(ovr.curve, CurveKind::Exp);
    }

    #[test]
    fn parse_layer_basic() {
        let src = r#"
layer reverb_wash {
    depth -> reverb_mix (0.0..0.8) smoothstep
    depth -> delay_mix (0.0..0.4) linear
}
track drums {
  kit: default
  section main [1 bars] {
    kick: [X . . .]
  }
}
"#;
        let prog = parse(src).unwrap();
        assert_eq!(prog.layers.len(), 1);
        assert_eq!(prog.layers[0].name, "reverb_wash");
        assert_eq!(prog.layers[0].mappings.len(), 2);
        assert!(!prog.layers[0].enabled_by_default);

        assert_eq!(prog.layers[0].mappings[0].macro_name, "depth");
        assert_eq!(prog.layers[0].mappings[0].target_param, "reverb_mix");
        assert_eq!(prog.layers[0].mappings[0].range, (0.0, 0.8));
        assert_eq!(prog.layers[0].mappings[0].curve, CurveKind::Smoothstep);

        assert_eq!(prog.layers[0].mappings[1].target_param, "delay_mix");
        assert_eq!(prog.layers[0].mappings[1].range, (0.0, 0.4));
        assert_eq!(prog.layers[0].mappings[1].curve, CurveKind::Linear);
    }

    #[test]
    fn parse_multiple_layers() {
        let src = r#"
layer reverb { depth -> mix (0.0..1.0) linear }
layer delay { time -> feedback (0.0..0.9) exp }
track drums {
  kit: default
  section main [1 bars] { kick: [X . . .] }
}
"#;
        let prog = parse(src).unwrap();
        assert_eq!(prog.layers.len(), 2);
        assert_eq!(prog.layers[0].name, "reverb");
        assert_eq!(prog.layers[1].name, "delay");
    }

    #[test]
    fn parse_layer_default_range_and_curve() {
        let src = r#"
layer fx {
    filter -> cutoff
}
track drums {
  kit: default
  section main [1 bars] { kick: [X . . .] }
}
"#;
        let prog = parse(src).unwrap();
        assert_eq!(prog.layers[0].mappings[0].range, (0.0, 1.0));
        assert_eq!(prog.layers[0].mappings[0].curve, CurveKind::Linear);
    }

    #[test]
    fn parse_no_layers_backward_compat() {
        let src = r#"
track drums {
  kit: default
  section main [1 bars] { kick: [X . . .] }
}
"#;
        let prog = parse(src).unwrap();
        assert!(prog.layers.is_empty());
    }

    // --- Functional chain syntax tests ---

    #[test]
    fn parse_functional_bass_track() {
        let src = r#"drums = bass() |> kick.pattern("X . . .")"#;
        let prog = parse(src).unwrap();
        assert_eq!(prog.tracks.len(), 1);
        assert_eq!(prog.tracks[0].name, "drums");
        assert_eq!(prog.tracks[0].instrument, InstrumentRef::Bass);
        assert_eq!(prog.tracks[0].sections.len(), 1);
        assert_eq!(prog.tracks[0].sections[0].name, "main");
    }

    #[test]
    fn parse_functional_poly_track() {
        let src = r#"pad = poly() |> note.pattern("X . X .")"#;
        let prog = parse(src).unwrap();
        assert_eq!(prog.tracks[0].instrument, InstrumentRef::Poly);
    }

    #[test]
    fn parse_functional_pluck_track() {
        let src = r#"keys = pluck() |> note.pattern("X . X .")"#;
        let prog = parse(src).unwrap();
        assert_eq!(prog.tracks[0].instrument, InstrumentRef::Pluck);
    }

    #[test]
    fn parse_functional_noise_track() {
        let src = r#"fx = noise() |> hit.pattern("X . . .")"#;
        let prog = parse(src).unwrap();
        assert_eq!(prog.tracks[0].instrument, InstrumentRef::Noise);
    }

    #[test]
    fn parse_functional_kit_track() {
        let src = r#"drums = kit("default") |> kick.pattern("X . . .")"#;
        let prog = parse(src).unwrap();
        assert_eq!(
            prog.tracks[0].instrument,
            InstrumentRef::Kit("default".to_string())
        );
    }

    #[test]
    fn parse_functional_instrument_only() {
        let src = "mytrack = bass()";
        let prog = parse(src).unwrap();
        assert_eq!(prog.tracks[0].name, "mytrack");
        assert_eq!(prog.tracks[0].instrument, InstrumentRef::Bass);
        assert!(prog.tracks[0].sections.is_empty());
    }

    #[test]
    fn parse_functional_inline_pattern_rest_char() {
        let src = r#"drums = kit("default") |> kick.pattern("X.x?")"#;
        let prog = parse(src).unwrap();
        let steps = &prog.tracks[0].sections[0].patterns[0].steps;
        assert_eq!(steps[0], Step::Hit);
        assert_eq!(steps[1], Step::Rest);
        assert_eq!(steps[2], Step::Accent(0.5));
        assert_eq!(steps[3], Step::Random(0.5)); // '?' is random with 50% probability
    }

    // --- Mapping log curve ---

    #[test]
    fn parse_mapping_log_curve() {
        let src = "map intensity -> drive (0.0..10.0) log";
        let prog = parse(src).unwrap();
        assert_eq!(prog.mappings[0].curve, CurveKind::Log);
    }

    // --- Override log curve ---

    #[test]
    fn parse_override_log_curve() {
        let src = r#"
track drums {
  kit: default
  section main [1 bars] {
    kick: [X . . .]
    override intensity -> drive (0.0..10.0) log
  }
}
"#;
        let prog = parse(src).unwrap();
        assert_eq!(
            prog.tracks[0].sections[0].overrides[0].curve,
            CurveKind::Log
        );
    }

    // --- Layer log curve ---

    #[test]
    fn parse_plugin_declarative() {
        let src = r#"
track lead {
  plugin: warm_pad
  section main [2 bars] {
    note: [C4 . . C4 . . E4 .]
  }
}
"#;
        let prog = parse(src).unwrap();
        assert_eq!(
            prog.tracks[0].instrument,
            InstrumentRef::Plugin("warm_pad".to_string())
        );
    }

    #[test]
    fn parse_plugin_functional() {
        let src = r#"lead = plugin("warm_pad") |> note.pattern("X . X .")"#;
        let prog = parse(src).unwrap();
        assert_eq!(
            prog.tracks[0].instrument,
            InstrumentRef::Plugin("warm_pad".to_string())
        );
        assert_eq!(prog.tracks[0].name, "lead");
    }

    #[test]
    fn parse_layer_log_curve() {
        let src = r#"
layer wash {
  intensity -> drive (0.0..10.0) log
}
track drums {
  kit: default
  section main [1 bars] { kick: [X . . .] }
}
"#;
        let prog = parse(src).unwrap();
        assert_eq!(prog.layers[0].mappings[0].curve, CurveKind::Log);
    }

    // --- Transform parsing tests ---

    #[test]
    fn parse_transform_fast() {
        let src = r#"drums = kit("default") |> kick.pattern("X.x.").fast(2)"#;
        let prog = parse(src).unwrap();
        let pat = &prog.tracks[0].sections[0].patterns[0];
        assert_eq!(pat.transforms.len(), 1);
        assert_eq!(pat.transforms[0], Transform::Fast(2.0));
    }

    #[test]
    fn parse_transform_slow() {
        let src = r#"drums = kit("default") |> kick.pattern("X.x.X.x.").slow(2)"#;
        let prog = parse(src).unwrap();
        let pat = &prog.tracks[0].sections[0].patterns[0];
        assert_eq!(pat.transforms[0], Transform::Slow(2.0));
    }

    #[test]
    fn parse_transform_rev() {
        let src = r#"drums = kit("default") |> kick.pattern("X.x.").rev()"#;
        let prog = parse(src).unwrap();
        let pat = &prog.tracks[0].sections[0].patterns[0];
        assert_eq!(pat.transforms[0], Transform::Rev);
    }

    #[test]
    fn parse_transform_rotate() {
        let src = r#"drums = kit("default") |> kick.pattern("X...").rotate(2)"#;
        let prog = parse(src).unwrap();
        let pat = &prog.tracks[0].sections[0].patterns[0];
        assert_eq!(pat.transforms[0], Transform::Rotate(2));
    }

    #[test]
    fn parse_transform_degrade() {
        let src = r#"drums = kit("default") |> kick.pattern("XXXX").degrade(0.3)"#;
        let prog = parse(src).unwrap();
        let pat = &prog.tracks[0].sections[0].patterns[0];
        assert_eq!(pat.transforms[0], Transform::Degrade(0.3));
    }

    #[test]
    fn parse_transform_chain_multiple() {
        let src = r#"drums = kit("default") |> kick.pattern("X.x.").fast(2).rev()"#;
        let prog = parse(src).unwrap();
        let pat = &prog.tracks[0].sections[0].patterns[0];
        assert_eq!(pat.transforms.len(), 2);
        assert_eq!(pat.transforms[0], Transform::Fast(2.0));
        assert_eq!(pat.transforms[1], Transform::Rev);
    }

    #[test]
    fn parse_transform_every() {
        let src = r#"drums = kit("default") |> kick.pattern("X.x.").every(4, rev)"#;
        let prog = parse(src).unwrap();
        let pat = &prog.tracks[0].sections[0].patterns[0];
        assert_eq!(
            pat.transforms[0],
            Transform::Every(4, Box::new(Transform::Rev))
        );
    }

    #[test]
    fn parse_transform_sometimes() {
        let src = r#"drums = kit("default") |> kick.pattern("X.x.").sometimes(0.5, fast(2))"#;
        let prog = parse(src).unwrap();
        let pat = &prog.tracks[0].sections[0].patterns[0];
        assert_eq!(
            pat.transforms[0],
            Transform::Sometimes(0.5, Box::new(Transform::Fast(2.0)))
        );
    }

    #[test]
    fn parse_transform_chop() {
        let src = r#"drums = kit("default") |> kick.pattern("X.x.").chop(4)"#;
        let prog = parse(src).unwrap();
        let pat = &prog.tracks[0].sections[0].patterns[0];
        assert_eq!(pat.transforms[0], Transform::Chop(4));
    }

    #[test]
    fn parse_transform_stutter() {
        let src = r#"drums = kit("default") |> kick.pattern("X.x.").stutter(3)"#;
        let prog = parse(src).unwrap();
        let pat = &prog.tracks[0].sections[0].patterns[0];
        assert_eq!(pat.transforms[0], Transform::Stutter(3));
    }

    #[test]
    fn parse_transform_add() {
        let src = r#"lead = bass() |> note.pattern("X.X.").add(7)"#;
        let prog = parse(src).unwrap();
        let pat = &prog.tracks[0].sections[0].patterns[0];
        assert_eq!(pat.transforms[0], Transform::Add(7));
    }

    #[test]
    fn parse_transform_gain() {
        let src = r#"drums = kit("default") |> kick.pattern("X.x.").gain(0.5)"#;
        let prog = parse(src).unwrap();
        let pat = &prog.tracks[0].sections[0].patterns[0];
        assert_eq!(pat.transforms[0], Transform::Gain(0.5));
    }

    #[test]
    fn parse_transform_legato() {
        let src = r#"lead = bass() |> note.pattern("X.X.").legato(2.0)"#;
        let prog = parse(src).unwrap();
        let pat = &prog.tracks[0].sections[0].patterns[0];
        assert_eq!(pat.transforms[0], Transform::Legato(2.0));
    }

    #[test]
    fn parse_no_transforms_backward_compat() {
        let src = r#"drums = kit("default") |> kick.pattern("X.x.")"#;
        let prog = parse(src).unwrap();
        let pat = &prog.tracks[0].sections[0].patterns[0];
        assert!(pat.transforms.is_empty());
    }

    #[test]
    fn parse_declarative_has_empty_transforms() {
        let src = r#"
track drums {
  kit: default
  section main [1 bars] {
    kick: [X . . .]
  }
}
"#;
        let prog = parse(src).unwrap();
        let pat = &prog.tracks[0].sections[0].patterns[0];
        assert!(pat.transforms.is_empty());
    }

    #[test]
    fn parse_transform_with_vel() {
        let src = r#"drums = kit("default") |> kick.pattern("X.x.").vel(0.8).fast(2)"#;
        let prog = parse(src).unwrap();
        let pat = &prog.tracks[0].sections[0].patterns[0];
        assert!(pat.velocities.is_some());
        assert_eq!(pat.transforms.len(), 1);
        assert_eq!(pat.transforms[0], Transform::Fast(2.0));
    }

    // --- Mini-notation tests ---

    #[test]
    fn parse_mini_notation_group_repeat() {
        let steps = parse_mini_notation("[X.]*3").unwrap();
        assert_eq!(steps.len(), 6);
        assert_eq!(steps[0], Step::Hit);
        assert_eq!(steps[1], Step::Rest);
        assert_eq!(steps[2], Step::Hit);
        assert_eq!(steps[3], Step::Rest);
        assert_eq!(steps[4], Step::Hit);
        assert_eq!(steps[5], Step::Rest);
    }

    #[test]
    fn parse_mini_notation_element_repeat() {
        let steps = parse_mini_notation("X!3").unwrap();
        assert_eq!(steps.len(), 3);
        assert!(steps.iter().all(|s| *s == Step::Hit));
    }

    #[test]
    fn parse_mini_notation_accent_group() {
        let steps = parse_mini_notation("[Xx]*2").unwrap();
        assert_eq!(steps.len(), 4);
        assert_eq!(steps[0], Step::Hit);
        assert_eq!(steps[1], Step::Accent(0.5));
        assert_eq!(steps[2], Step::Hit);
        assert_eq!(steps[3], Step::Accent(0.5));
    }

    #[test]
    fn parse_mini_notation_rest_group() {
        let steps = parse_mini_notation("[.]*4").unwrap();
        assert_eq!(steps.len(), 4);
        assert!(steps.iter().all(|s| *s == Step::Rest));
    }

    #[test]
    fn parse_mini_notation_rest_element_repeat() {
        let steps = parse_mini_notation(".!3").unwrap();
        assert_eq!(steps.len(), 3);
        assert!(steps.iter().all(|s| *s == Step::Rest));
    }

    #[test]
    fn parse_mini_notation_identity_repeat() {
        let steps = parse_mini_notation("X!1").unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0], Step::Hit);
    }

    #[test]
    fn parse_mini_notation_mixed() {
        let steps = parse_mini_notation("[X.]*2.x").unwrap();
        assert_eq!(steps.len(), 6);
        assert_eq!(steps[0], Step::Hit);
        assert_eq!(steps[1], Step::Rest);
        assert_eq!(steps[2], Step::Hit);
        assert_eq!(steps[3], Step::Rest);
        assert_eq!(steps[4], Step::Rest);
        assert_eq!(steps[5], Step::Accent(0.5));
    }

    #[test]
    fn parse_mini_notation_backward_compat() {
        let steps = parse_mini_notation("X.x.").unwrap();
        assert_eq!(steps.len(), 4);
        assert_eq!(steps[0], Step::Hit);
        assert_eq!(steps[1], Step::Rest);
        assert_eq!(steps[2], Step::Accent(0.5));
        assert_eq!(steps[3], Step::Rest);
    }

    #[test]
    fn parse_mini_notation_nested_group_with_trailing() {
        let steps = parse_mini_notation("[X.x]*2!").unwrap();
        // [X.x]*2 = X.xX.x (6 steps), then '!' is unknown char → rest
        assert_eq!(steps.len(), 7);
        assert_eq!(steps[0], Step::Hit);
        assert_eq!(steps[1], Step::Rest);
        assert_eq!(steps[2], Step::Accent(0.5));
        assert_eq!(steps[3], Step::Hit);
        assert_eq!(steps[4], Step::Rest);
        assert_eq!(steps[5], Step::Accent(0.5));
        assert_eq!(steps[6], Step::Rest); // '!' with no digit → treated as unknown
    }

    #[test]
    fn parse_mini_notation_unmatched_bracket() {
        let result = parse_mini_notation("[X.");
        assert!(result.is_err());
    }

    #[test]
    fn parse_mini_notation_full_round_trip() {
        let src = r#"drums = kit("default") |> kick.pattern("[X.]*3X!2")"#;
        let prog = parse(src).unwrap();
        let steps = &prog.tracks[0].sections[0].patterns[0].steps;
        // [X.]*3 = X.X.X. (6) + X!2 = XX (2) = 8 total
        assert_eq!(steps.len(), 8);
        assert_eq!(steps[0], Step::Hit);
        assert_eq!(steps[1], Step::Rest);
        assert_eq!(steps[6], Step::Hit);
        assert_eq!(steps[7], Step::Hit);
    }

    #[test]
    fn parse_mini_notation_compile_round_trip() {
        use crate::dsl::compile::compile_program;
        let src = r#"
tempo 120
drums = kit("default") |> kick.pattern("[X.]*2")
"#;
        let prog = parse(src).unwrap();
        let song = compile_program(&prog).unwrap();
        // [X.]*2 = X.X. = 4 steps, 2 hits → 2 events
        assert_eq!(song.events.len(), 2);
    }

    // --- Extended mini-notation tests ---

    #[test]
    fn parse_mini_euclidean_3_8() {
        let steps = parse_mini_notation("E(3,8)").unwrap();
        assert_eq!(steps.len(), 8);
        let hits: usize = steps.iter().filter(|s| **s == Step::Hit).count();
        assert_eq!(hits, 3);
    }

    #[test]
    fn parse_mini_euclidean_5_8() {
        let steps = parse_mini_notation("E(5,8)").unwrap();
        assert_eq!(steps.len(), 8);
        let hits: usize = steps.iter().filter(|s| **s == Step::Hit).count();
        assert_eq!(hits, 5);
    }

    #[test]
    fn parse_mini_euclidean_rotated() {
        let base = parse_mini_notation("E(3,8)").unwrap();
        let rotated = parse_mini_notation("E(3,8,1)").unwrap();
        assert_eq!(rotated.len(), 8);
        let hits: usize = rotated.iter().filter(|s| **s == Step::Hit).count();
        assert_eq!(hits, 3);
        // Rotation should produce a different pattern
        assert_ne!(base, rotated);
    }

    #[test]
    fn parse_mini_euclidean_edge_cases() {
        // All hits
        let all = parse_mini_notation("E(4,4)").unwrap();
        assert_eq!(all.len(), 4);
        assert!(all.iter().all(|s| *s == Step::Hit));

        // No hits
        let none = parse_mini_notation("E(0,4)").unwrap();
        assert_eq!(none.len(), 4);
        assert!(none.iter().all(|s| *s == Step::Rest));
    }

    #[test]
    fn parse_mini_random_default() {
        let steps = parse_mini_notation("?").unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0], Step::Random(0.5));
    }

    #[test]
    fn parse_mini_random_with_probability() {
        let steps = parse_mini_notation("?0.7").unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0], Step::Random(0.7));
    }

    #[test]
    fn parse_mini_alternate() {
        let steps = parse_mini_notation("<X x .>").unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(
            steps[0],
            Step::Alternate(vec![Step::Hit, Step::Accent(0.5), Step::Rest])
        );
    }

    #[test]
    fn parse_mini_subdivided() {
        let steps = parse_mini_notation("{X.x}").unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(
            steps[0],
            Step::Subdivided(vec![Step::Hit, Step::Rest, Step::Accent(0.5)])
        );
    }

    #[test]
    fn parse_mini_ratchet() {
        let steps = parse_mini_notation("X^3").unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0], Step::Ratchet(Box::new(Step::Hit), 3));
    }

    #[test]
    fn parse_mini_mixed_extended() {
        let steps = parse_mini_notation("E(3,8)?X^2").unwrap();
        // E(3,8) = 8 steps + ? = 1 random + X^2 = 1 ratchet = 10 total
        assert_eq!(steps.len(), 10);
        // First 8 are euclidean (3 hits, 5 rests)
        let eucl_hits: usize = steps[..8].iter().filter(|s| **s == Step::Hit).count();
        assert_eq!(eucl_hits, 3);
        // Step 8 is random
        assert_eq!(steps[8], Step::Random(0.5));
        // Step 9 is ratchet
        assert_eq!(steps[9], Step::Ratchet(Box::new(Step::Hit), 2));
    }

    #[test]
    fn parse_mini_unmatched_angle_bracket() {
        let result = parse_mini_notation("<X x");
        assert!(result.is_err());
    }

    #[test]
    fn parse_mini_unmatched_curly_brace() {
        let result = parse_mini_notation("{X.x");
        assert!(result.is_err());
    }
}
