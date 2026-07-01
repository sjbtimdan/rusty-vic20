//! Two-pass NMOS 6502 assembler. Reads AS65-syntax source, produces `Vec<u8>`.
//!
//! Supported syntax:
//! - All official 6502 opcodes + known illegal opcodes (DCP, ISC, LAX, RLA, RRA,
//!   SAX, SLO, SRE, ALR, ANC, ARR)
//! - All standard addressing modes, with automatic zero-page vs. absolute
//!   selection when the operand value is known
//! - Labels (`name` or `name:`)
//! - Directives: `org`, `db`/`.db`/`byte`, `dw`/`.dw`/`word`, `ds`/`.ds`,
//!   `equ`/`=`
//! - Expressions: hex (`$FF`), decimal, binary (`%0001`), char (`'A'`),
//!   `*` (current PC), `hi()`/`lo()`, `+`, `-`, `&`, `|`, `~`, parentheses
//! - Comments (`;` to end of line)
//! - `end` (stops assembly)
//!
//! # Example
//!
//! ```ignore
//! use nmos6502::assembler::assemble;
//!
//! let source = r#"
//!     org $1000
//!     lda #$42
//!     sta $2000
//!     brk
//! "#;
//! let (bytes, _symbols) = assemble(source, 0, None).unwrap();
//! assert_eq!(bytes, vec![0xA9, 0x42, 0x8D, 0x00, 0x20, 0x00]);
//! ```

use std::collections::HashMap;

use crate::opcode::{AddressingMode, Mnemonic, OPCODES};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Assemble 6502 source text into raw bytes.
///
/// * `source` — the `.a65` file contents
/// * `default_origin` — load address when no `org` is present (or origin before
///   first `org`)
/// * `predefined` — optional pre-populated symbol table (for constants such as
///   `code_segment = $400`)
pub fn assemble(
    source: &str,
    default_origin: u16,
    predefined: Option<HashMap<String, u16>>,
) -> Result<(Vec<u8>, HashMap<String, u16>), AssemblerError> {
    let mut asm = Assembler::new(default_origin, predefined);
    asm.assemble(source)?;
    Ok((std::mem::take(&mut asm.output), asm.symbols.clone()))
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssemblerError {
    UnknownOpcode(String, usize),
    UnknownLabel(String, usize),
    InvalidOperand(String, usize),
    ExpressionError(String, usize),
    DuplicateLabel(String, usize),
    AddressOverflow,
    ParseError(String, usize),
    MissingOperand(String, usize),
    UnexpectedEnd(usize),
}

impl std::fmt::Display for AssemblerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownOpcode(s, l) => write!(f, "line {l}: unknown opcode `{s}`"),
            Self::UnknownLabel(s, l) => write!(f, "line {l}: unknown label `{s}`"),
            Self::InvalidOperand(s, l) => write!(f, "line {l}: invalid operand `{s}`"),
            Self::ExpressionError(s, l) => write!(f, "line {l}: expression error: {s}"),
            Self::DuplicateLabel(s, l) => write!(f, "line {l}: duplicate label `{s}`"),
            Self::AddressOverflow => write!(f, "address overflow (> 0xFFFF)"),
            Self::ParseError(s, l) => write!(f, "line {l}: parse error: {s}"),
            Self::MissingOperand(s, l) => write!(f, "line {l}: {s} requires an operand"),
            Self::UnexpectedEnd(l) => write!(f, "line {l}: unexpected end of expression"),
        }
    }
}

// ---------------------------------------------------------------------------
// Token for the expression evaluator
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token<'a> {
    Number(u16),
    Plus,
    Minus,
    BitAnd,
    BitOr,
    BitNot,
    BitXor,
    Star, // current PC
    LParen,
    RParen,
    Comma,
    Ident(&'a str),
    Hi,
    Lo,
    // Comparison operators (used in `if` conditions)
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    Eof,
}

// ---------------------------------------------------------------------------
// Lines produced by the pre-processor
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ParsedLine {
    label: Option<String>,
    kind: LineKind,
    line: usize,
    operand: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LineKind {
    Org,
    Equ,
    Ds,
    Db,
    Dw,
    Instruction(Mnemonic),
    End,
    If(String),
    Else,
    Endif,
    Skip,
}

// ---------------------------------------------------------------------------
// Assembler
// ---------------------------------------------------------------------------

struct Assembler {
    pc: u16,
    star_pc: u16, // PC at the start of the current instruction/data (for `*`)
    default_origin: u16,
    symbols: HashMap<String, u16>,
    /// Set of symbol names that came from the predefined table (user source
    /// may override them without triggering a duplicate-label error).
    predefined_syms: std::collections::HashSet<String>,
    opcode_map: HashMap<(Mnemonic, AddressingMode), u8>,
    output: Vec<u8>,
    origin_seen: bool,
    /// Cache of (line_number → AddressingMode) from pass1, so pass2 reuses
    /// the same mode (preventing size mismatches for forward references).
    mode_cache: HashMap<usize, AddressingMode>,
    /// Nesting stack for `if`/`else`/`endif` — each entry is `true` when
    /// we are inside an active (taken) branch.
    if_stack: Vec<bool>,
}

impl Assembler {
    fn new(default_origin: u16, predefined: Option<HashMap<String, u16>>) -> Self {
        let mut symbols = predefined.unwrap_or_default();
        // Pre-populate common AS65 status-flag constants so the functional
        // test suite can resolve them immediately.
        symbols.entry("carry".into()).or_insert(0x01);
        symbols.entry("zero".into()).or_insert(0x02);
        symbols.entry("intdis".into()).or_insert(0x04);
        symbols.entry("decmode".into()).or_insert(0x08);
        symbols.entry("break".into()).or_insert(0x10);
        symbols.entry("reserv".into()).or_insert(0x20);
        symbols.entry("overfl".into()).or_insert(0x40);
        symbols.entry("minus".into()).or_insert(0x80);

        let predefined_names: std::collections::HashSet<String> = symbols.keys().cloned().collect();

        Self {
            pc: default_origin,
            star_pc: default_origin,
            default_origin,
            symbols,
            predefined_syms: predefined_names,
            opcode_map: build_opcode_map(),
            output: Vec::new(),
            origin_seen: false,
            mode_cache: HashMap::new(),
            if_stack: Vec::new(),
        }
    }

    // ------------------------------------------------------------------
    // Top-level two-pass
    // ------------------------------------------------------------------

    fn assemble(&mut self, source: &str) -> Result<(), AssemblerError> {
        // Expand macros at text level before the main two-pass assembly.
        let expanded = expand_source(source)?;
        let lines = preprocess(&expanded);
        self.pass1(&lines)?;
        self.pass2(&lines)
    }

    // ------------------------------------------------------------------
    // Pass 1 — build symbol table, compute instruction sizes
    // ------------------------------------------------------------------

    /// True when we are not inside a skipped `if`/`else`/`endif` block.
    fn is_active(&self) -> bool {
        self.if_stack.iter().all(|&active| active)
    }

    fn pass1(&mut self, lines: &[ParsedLine]) -> Result<(), AssemblerError> {
        self.pc = self.default_origin;
        self.origin_seen = false;
        self.if_stack.clear();

        for line in lines {
            // Handle conditional directives even when inactive.
            match &line.kind {
                LineKind::If(cond) => {
                    let val = self.eval_cond(cond, line.line);
                    self.if_stack.push(val != 0);
                    continue; // no bytes, no label
                }
                LineKind::Else => {
                    if let Some(top) = self.if_stack.last_mut() {
                        *top = !*top;
                    }
                    continue;
                }
                LineKind::Endif => {
                    self.if_stack.pop();
                    continue;
                }
                _ => {}
            }

            if !self.is_active() {
                continue;
            }

            // Resolve instruction size BEFORE registering the label
            // (so the label address is correct).
            match line.kind {
                LineKind::Org => {
                    self.pc = self.eval(&line.operand, line.line)?;
                    self.origin_seen = true;
                }
                LineKind::Equ => {
                    self.star_pc = self.pc; // `*` resolves to current PC
                    let val = self.eval(&line.operand, line.line)?;
                    if let Some(ref label) = line.label {
                        let key = label.to_ascii_lowercase();
                        // Equ redefinitions are always allowed in AS65
                        // (e.g. `test_num = test_num + 1`).
                        self.symbols.insert(key, val);
                    }
                }
                LineKind::Ds => {
                    let count = self.eval(&line.operand, line.line)?;
                    self.pc = self.pc.wrapping_add(count);
                }
                LineKind::Db => {
                    let items = split_values(&line.operand);
                    self.pc = self.pc.wrapping_add(items.len() as u16);
                }
                LineKind::Dw => {
                    let items = split_values(&line.operand);
                    self.pc = self.pc.wrapping_add((items.len() * 2) as u16);
                }
                LineKind::Instruction(mnemonic) => {
                    // Use emit=true so known values get proper ZP detection,
                    // but forward references still fall back to Absolute.
                    // Set star_pc BEFORE detect_mode so `*` resolves to the
                    // instruction start address (e.g. `jmp *` → self-trap).
                    self.star_pc = self.pc;
                    let mode = self.detect_mode(&line.operand, mnemonic, line.line, true)?;
                    self.mode_cache.insert(line.line, mode);
                    self.pc = self.pc.wrapping_add(mode_size(mode) as u16);
                }
                LineKind::End => break,
                _ => {}
            }

            // Register label with the PC address BEFORE this line's content.
            // Equ labels are already registered above; skip non-active branches.
            if self.is_active() {
                if let Some(ref label) = line.label {
                    let key = label.to_ascii_lowercase();
                    if line.kind != LineKind::Equ {
                        // Predefined symbols may be overridden by the source.
                        if self.symbols.contains_key(&key) && !self.predefined_syms.contains(&key) {
                            return Err(AssemblerError::DuplicateLabel(label.clone(), line.line));
                        }
                        // Label points to the address *before* this line consumed space.
                        let addr = self.pc - line_size(line, self).unwrap_or(0);
                        self.symbols.insert(key, addr);
                    }
                }
            }
        }

        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 2 — emit bytes
    // ------------------------------------------------------------------

    fn pass2(&mut self, lines: &[ParsedLine]) -> Result<(), AssemblerError> {
        self.pc = self.default_origin;
        self.output.clear();
        self.origin_seen = false;
        self.if_stack.clear();

        for line in lines {
            // Handle conditional directives even when inactive.
            match &line.kind {
                LineKind::If(cond) => {
                    let val = self.eval_cond(cond, line.line);
                    self.if_stack.push(val != 0);
                    continue;
                }
                LineKind::Else => {
                    if let Some(top) = self.if_stack.last_mut() {
                        *top = !*top;
                    }
                    continue;
                }
                LineKind::Endif => {
                    self.if_stack.pop();
                    continue;
                }
                _ => {}
            }

            if !self.is_active() {
                continue;
            }

            match &line.kind {
                LineKind::Org => {
                    let addr = self.eval(&line.operand, line.line)?;
                    if addr < self.pc && self.origin_seen {
                        return Err(AssemblerError::AddressOverflow);
                    }
                    // Pad with zeros from current position up to new origin.
                    let new_offset = addr.wrapping_sub(self.default_origin) as usize;
                    while self.output.len() < new_offset {
                        self.output.push(0);
                        self.pc = self.pc.wrapping_add(1);
                    }
                    self.pc = addr;
                    self.origin_seen = true;
                }
                LineKind::Equ => {
                    self.star_pc = self.pc; // `*` resolves to current PC
                    let val = self.eval(&line.operand, line.line)?;
                    if let Some(ref label) = line.label {
                        let key = label.to_ascii_lowercase();
                        self.symbols.insert(key, val);
                    }
                }
                LineKind::Ds => {
                    self.star_pc = self.pc;
                    let count = self.eval(&line.operand, line.line)? as usize;
                    self.output.resize(self.output.len() + count, 0);
                    self.pc = self.pc.wrapping_add(count as u16);
                }
                LineKind::Db => {
                    self.star_pc = self.pc;
                    for v in split_values(&line.operand) {
                        let byte = self.eval(v, line.line)? as u8;
                        self.output.push(byte);
                        self.pc = self.pc.wrapping_add(1);
                    }
                }
                LineKind::Dw => {
                    self.star_pc = self.pc;
                    for v in split_values(&line.operand) {
                        let word = self.eval(v, line.line)?;
                        self.output.push((word & 0xFF) as u8);
                        self.output.push((word >> 8) as u8);
                        self.pc = self.pc.wrapping_add(2);
                    }
                }
                LineKind::Instruction(mnemonic) => {
                    self.emit_instruction(*mnemonic, &line.operand, line.line)?;
                }
                LineKind::End => break,
                _ => {}
            }
        }

        Ok(())
    }

    // ------------------------------------------------------------------
    // Instruction emission
    // ------------------------------------------------------------------

    fn emit_instruction(&mut self, mnemonic: Mnemonic, operand: &str, line: usize) -> Result<(), AssemblerError> {
        // Use the cached mode from pass1 to guarantee consistent sizes.
        let mode = self
            .mode_cache
            .get(&line)
            .copied()
            .or_else(|| {
                // Fallback if not cached (shouldn't happen for well-formed input).
                self.detect_mode(operand, mnemonic, line, true).ok()
            })
            .ok_or_else(|| AssemblerError::UnknownOpcode(format!("{:?} with operand `{operand}`", mnemonic), line))?;
        let opcode = self.lookup_opcode(mnemonic, mode, line)?;
        self.star_pc = self.pc; // `*` resolves to the instruction start address
        self.output.push(opcode);
        self.pc = self.pc.wrapping_add(1);

        match mode {
            AddressingMode::Implied | AddressingMode::Accumulator => {}
            AddressingMode::Immediate
            | AddressingMode::ZeroPage
            | AddressingMode::ZeroPageX
            | AddressingMode::ZeroPageY
            | AddressingMode::IndexedIndirect
            | AddressingMode::IndirectIndexed => {
                let val = self.extract_operand(operand, mode, line)? as u8;
                self.output.push(val);
                self.pc = self.pc.wrapping_add(1);
            }
            AddressingMode::Relative => {
                let target = self.extract_operand(operand, mode, line)?;
                // Relative offset is from the NEXT instruction (PC + 2).
                // pc is currently past the opcode (+1), so add 1 more.
                let next_pc = self.pc.wrapping_add(1);
                let offset = target.wrapping_sub(next_pc) as i16 as i8;
                self.output.push(offset as u8);
                self.pc = next_pc;
            }
            AddressingMode::Absolute
            | AddressingMode::AbsoluteX
            | AddressingMode::AbsoluteY
            | AddressingMode::Indirect => {
                let val = self.extract_operand(operand, mode, line)?;
                self.output.push((val & 0xFF) as u8);
                self.output.push((val >> 8) as u8);
                self.pc = self.pc.wrapping_add(2);
            }
        }

        Ok(())
    }

    // ------------------------------------------------------------------
    // Addressing-mode detection
    // ------------------------------------------------------------------

    fn detect_mode(
        &self,
        operand: &str,
        mnemonic: Mnemonic,
        line: usize,
        emit: bool,
    ) -> Result<AddressingMode, AssemblerError> {
        let op = operand.trim();

        // --- No operand or accumulator shorthand ---
        if op.is_empty() || op.eq_ignore_ascii_case("a") {
            return match mnemonic {
                Mnemonic::Asl | Mnemonic::Lsr | Mnemonic::Rol | Mnemonic::Ror => Ok(AddressingMode::Accumulator),
                _ => Ok(AddressingMode::Implied),
            };
        }

        // --- Immediate (#) ---
        if op.starts_with('#') {
            return Ok(AddressingMode::Immediate);
        }

        // --- Indirect forms ---
        if op.starts_with('(') {
            if op.contains(",X") || op.contains(",x") {
                return Ok(AddressingMode::IndexedIndirect);
            }
            if op.contains("),Y") || op.contains("),y") {
                return Ok(AddressingMode::IndirectIndexed);
            }
            return Ok(AddressingMode::Indirect);
        }

        // --- Indexed (,X or ,Y) ---
        let (has_x, has_y) = (
            op.ends_with(",X") || op.ends_with(",x"),
            op.ends_with(",Y") || op.ends_with(",y"),
        );

        if has_x || has_y {
            let stripped = if has_x {
                op.trim_end_matches(",X").trim_end_matches(",x")
            } else {
                op.trim_end_matches(",Y").trim_end_matches(",y")
            };
            let inner = stripped.trim();

            let zp_mode = if has_x {
                AddressingMode::ZeroPageX
            } else {
                AddressingMode::ZeroPageY
            };
            let abs_mode = if has_x {
                AddressingMode::AbsoluteX
            } else {
                AddressingMode::AbsoluteY
            };

            // Prefer zero-page when address fits AND opcode exists.
            let fits = emit && self.value_fits_zp(inner, line)?;
            if fits && opcode_exists(mnemonic, zp_mode) {
                return Ok(zp_mode);
            }
            // Fall back to absolute indexed if opcode exists there.
            if opcode_exists(mnemonic, abs_mode) {
                return Ok(abs_mode);
            }
            // Last resort — return zero-page mode (will error at lookup
            // if the byte doesn't fit, which is correct for invalid code).
            return Ok(zp_mode);
        }

        // --- Bare expression ---
        if is_branch(mnemonic) {
            return Ok(AddressingMode::Relative);
        }

        if emit && self.value_fits_zp(op, line)? {
            Ok(AddressingMode::ZeroPage)
        } else {
            Ok(AddressingMode::Absolute)
        }
    }

    /// True if the expression evaluates to ≤ 0xFF (can use zero-page).
    /// Forward references are assumed to NOT fit (safe default).
    fn value_fits_zp(&self, expr: &str, line: usize) -> Result<bool, AssemblerError> {
        match self.eval(expr, line) {
            Ok(v) => Ok(v <= 0xFF),
            Err(AssemblerError::UnknownLabel(_, _)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Extract the numeric operand value from an instruction operand based on
    /// the detected addressing mode.
    fn extract_operand(&self, operand: &str, mode: AddressingMode, line: usize) -> Result<u16, AssemblerError> {
        let op = operand.trim();
        match mode {
            AddressingMode::Immediate => self.eval(op.trim_start_matches('#').trim(), line),
            AddressingMode::IndexedIndirect => {
                // ($44,X) → extract $44
                let inner = op
                    .trim_start_matches('(')
                    .trim_end_matches(",X")
                    .trim_end_matches(",x")
                    .trim_end_matches(')')
                    .trim();
                self.eval(inner, line)
            }
            AddressingMode::IndirectIndexed => {
                // ($44),Y → extract $44
                let inner = op
                    .trim_start_matches('(')
                    .trim_end_matches("),Y")
                    .trim_end_matches("),y")
                    .trim();
                self.eval(inner, line)
            }
            AddressingMode::Indirect => {
                // ($2000) → extract $2000
                let inner = op.trim_start_matches('(').trim_end_matches(')').trim();
                self.eval(inner, line)
            }
            AddressingMode::ZeroPageX | AddressingMode::AbsoluteX => {
                self.eval(op.trim_end_matches(",X").trim_end_matches(",x").trim(), line)
            }
            AddressingMode::ZeroPageY | AddressingMode::AbsoluteY => {
                self.eval(op.trim_end_matches(",Y").trim_end_matches(",y").trim(), line)
            }
            AddressingMode::Relative | AddressingMode::ZeroPage | AddressingMode::Absolute => self.eval(op, line),
            _ => Err(AssemblerError::InvalidOperand(operand.into(), line)),
        }
    }

    // ------------------------------------------------------------------
    // Opcode lookup
    // ------------------------------------------------------------------

    fn lookup_opcode(&self, mnemonic: Mnemonic, mode: AddressingMode, line: usize) -> Result<u8, AssemblerError> {
        // NOP implied always uses the canonical 0xEA.
        if mnemonic == Mnemonic::Nop && mode == AddressingMode::Implied {
            return Ok(0xEA);
        }

        self.opcode_map
            .get(&(mnemonic, mode))
            .copied()
            .ok_or_else(|| AssemblerError::UnknownOpcode(format!("{:?} {:?}", mnemonic, mode), line))
    }

    // ------------------------------------------------------------------
    // Expression evaluator (recursive descent)
    // ------------------------------------------------------------------

    fn eval(&self, expr: &str, line: usize) -> Result<u16, AssemblerError> {
        eval_expr(expr, &self.symbols, self.star_pc, line)
    }

    /// Evaluate an `if` condition, treating unknown labels as 0 (false-y).
    fn eval_cond(&self, expr: &str, line: usize) -> u16 {
        eval_cond_expr(expr, &self.symbols, line)
    }
}

// ---------------------------------------------------------------------------
// Standalone expression evaluator (used by Assembler and MacroExpander)
// ---------------------------------------------------------------------------

/// Tokenize an expression string into a token vec.
fn tokenize_expr<'a>(s: &'a str, line: usize) -> Result<Vec<Token<'a>>, AssemblerError> {
    let mut tokens = Vec::new();
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let ch = bytes[i] as char;
        match ch {
            ' ' | '\t' => i += 1,
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '&' => {
                tokens.push(Token::BitAnd);
                i += 1;
            }
            '|' => {
                tokens.push(Token::BitOr);
                i += 1;
            }
            '~' => {
                tokens.push(Token::BitNot);
                i += 1;
            }
            '^' => {
                tokens.push(Token::BitXor);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '!' => {
                if i + 1 < len && bytes[i + 1] == b'=' {
                    tokens.push(Token::Ne);
                    i += 2;
                } else {
                    return Err(AssemblerError::ExpressionError(
                        "unexpected '!' — use '~' for bitwise not".into(),
                        line,
                    ));
                }
            }
            '=' => {
                tokens.push(Token::Eq);
                i += 1;
            }
            '<' => {
                if i + 1 < len && bytes[i + 1] == b'=' {
                    tokens.push(Token::Le);
                    i += 2;
                } else {
                    tokens.push(Token::Lt);
                    i += 1;
                }
            }
            '>' => {
                if i + 1 < len && bytes[i + 1] == b'=' {
                    tokens.push(Token::Ge);
                    i += 2;
                } else {
                    tokens.push(Token::Gt);
                    i += 1;
                }
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            '$' => {
                i += 1;
                let start = i;
                while i < len && (bytes[i] as char).is_ascii_hexdigit() {
                    i += 1;
                }
                if i == start {
                    return Err(AssemblerError::ExpressionError(
                        "empty hex literal after $".into(),
                        line,
                    ));
                }
                let hex = &s[start..i];
                let v = u16::from_str_radix(hex, 16)
                    .map_err(|_| AssemblerError::ExpressionError(format!("invalid hex ${hex}"), line))?;
                tokens.push(Token::Number(v));
            }
            '%' => {
                i += 1;
                let start = i;
                while i < len && matches!(bytes[i] as char, '0' | '1') {
                    i += 1;
                }
                if i == start {
                    return Err(AssemblerError::ExpressionError(
                        "empty binary literal after %".into(),
                        line,
                    ));
                }
                let mut val: u16 = 0;
                for &b in &s.as_bytes()[start..i] {
                    val = (val << 1) | (if b == b'1' { 1 } else { 0 });
                }
                tokens.push(Token::Number(val));
            }
            '\'' => {
                i += 1; // opening quote
                if i >= len {
                    return Err(AssemblerError::ExpressionError(
                        "unclosed character literal".into(),
                        line,
                    ));
                }
                let c = bytes[i];
                i += 1;
                if i >= len || bytes[i] as char != '\'' {
                    return Err(AssemblerError::ExpressionError(
                        "unclosed character literal".into(),
                        line,
                    ));
                }
                i += 1; // closing quote
                tokens.push(Token::Number(c as u16));
            }
            '0'..='9' => {
                let start = i;
                while i < len && (bytes[i] as char).is_ascii_digit() {
                    i += 1;
                }
                let num = &s[start..i];
                let val: u32 = num
                    .parse()
                    .map_err(|_| AssemblerError::ExpressionError(format!("invalid number `{num}`"), line))?;
                if val > 0xFFFF {
                    return Err(AssemblerError::ExpressionError("decimal value > 0xFFFF".into(), line));
                }
                tokens.push(Token::Number(val as u16));
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let start = i;
                while i < len {
                    let c = bytes[i] as char;
                    if c.is_alphanumeric() || c == '_' || c == '?' {
                        i += 1;
                    } else {
                        break;
                    }
                }
                let ident = &s[start..i];
                match ident.to_ascii_lowercase().as_str() {
                    "hi" => tokens.push(Token::Hi),
                    "lo" => tokens.push(Token::Lo),
                    _ => tokens.push(Token::Ident(ident)),
                }
            }
            _ => {
                return Err(AssemblerError::ExpressionError(
                    format!("unexpected character '{ch}'"),
                    line,
                ));
            }
        }
    }

    tokens.push(Token::Eof);
    Ok(tokens)
}

// -- Recursive-descent parser ---------------------------------------
//
// Precedence (highest to lowest):
//   unary '-', '~'
//   '*'
//   '+', '-'
//   '=', '!=', '<', '>', '<=', '>='
//   '&', '|', '^'

fn parse_expr(
    toks: &[Token],
    pos: usize,
    symbols: &HashMap<String, u16>,
    star_pc: u16,
    line: usize,
) -> Result<(u16, usize), AssemblerError> {
    let (mut left, mut p) = parse_comparison(toks, pos, symbols, star_pc, line)?;
    while p < toks.len() {
        match toks[p] {
            Token::BitAnd => {
                let (r, q) = parse_comparison(toks, p + 1, symbols, star_pc, line)?;
                left &= r;
                p = q;
            }
            Token::BitOr => {
                let (r, q) = parse_comparison(toks, p + 1, symbols, star_pc, line)?;
                left |= r;
                p = q;
            }
            Token::BitXor => {
                let (r, q) = parse_comparison(toks, p + 1, symbols, star_pc, line)?;
                left ^= r;
                p = q;
            }
            _ => break,
        }
    }
    Ok((left, p))
}

fn parse_comparison(
    toks: &[Token],
    pos: usize,
    symbols: &HashMap<String, u16>,
    star_pc: u16,
    line: usize,
) -> Result<(u16, usize), AssemblerError> {
    let (mut left, mut p) = parse_additive(toks, pos, symbols, star_pc, line)?;
    while p < toks.len() {
        match toks[p] {
            Token::Eq => {
                let (r, q) = parse_additive(toks, p + 1, symbols, star_pc, line)?;
                left = if left == r { 1 } else { 0 };
                p = q;
            }
            Token::Ne => {
                let (r, q) = parse_additive(toks, p + 1, symbols, star_pc, line)?;
                left = if left != r { 1 } else { 0 };
                p = q;
            }
            Token::Lt => {
                let (r, q) = parse_additive(toks, p + 1, symbols, star_pc, line)?;
                left = if left < r { 1 } else { 0 };
                p = q;
            }
            Token::Gt => {
                let (r, q) = parse_additive(toks, p + 1, symbols, star_pc, line)?;
                left = if left > r { 1 } else { 0 };
                p = q;
            }
            Token::Le => {
                let (r, q) = parse_additive(toks, p + 1, symbols, star_pc, line)?;
                left = if left <= r { 1 } else { 0 };
                p = q;
            }
            Token::Ge => {
                let (r, q) = parse_additive(toks, p + 1, symbols, star_pc, line)?;
                left = if left >= r { 1 } else { 0 };
                p = q;
            }
            _ => break,
        }
    }
    Ok((left, p))
}

// additive = term (('+' | '-') term)*
fn parse_additive(
    toks: &[Token],
    pos: usize,
    symbols: &HashMap<String, u16>,
    star_pc: u16,
    line: usize,
) -> Result<(u16, usize), AssemblerError> {
    let (mut left, mut p) = parse_term(toks, pos, symbols, star_pc, line)?;
    while p < toks.len() {
        match toks[p] {
            Token::Plus => {
                let (r, q) = parse_term(toks, p + 1, symbols, star_pc, line)?;
                left = left.wrapping_add(r);
                p = q;
            }
            Token::Minus => {
                let (r, q) = parse_term(toks, p + 1, symbols, star_pc, line)?;
                left = left.wrapping_sub(r);
                p = q;
            }
            _ => break,
        }
    }
    Ok((left, p))
}

// term    = unary (('*') unary)*
fn parse_term(
    toks: &[Token],
    pos: usize,
    symbols: &HashMap<String, u16>,
    star_pc: u16,
    line: usize,
) -> Result<(u16, usize), AssemblerError> {
    let (mut left, mut p) = parse_unary(toks, pos, symbols, star_pc, line)?;
    while p < toks.len() {
        match toks[p] {
            Token::Star => {
                // '*' as binary operator → multiplication
                let (r, q) = parse_unary(toks, p + 1, symbols, star_pc, line)?;
                left = left.wrapping_mul(r);
                p = q;
            }
            _ => break,
        }
    }
    Ok((left, p))
}

// unary   = '-' unary | '~' unary | primary
fn parse_unary(
    toks: &[Token],
    pos: usize,
    symbols: &HashMap<String, u16>,
    star_pc: u16,
    line: usize,
) -> Result<(u16, usize), AssemblerError> {
    if pos >= toks.len() {
        return Err(AssemblerError::UnexpectedEnd(line));
    }
    match toks[pos] {
        Token::Minus => {
            let (v, p) = parse_unary(toks, pos + 1, symbols, star_pc, line)?;
            Ok(((!v).wrapping_add(1), p))
        }
        Token::BitNot => {
            let (v, p) = parse_unary(toks, pos + 1, symbols, star_pc, line)?;
            Ok((!v, p))
        }
        _ => parse_primary(toks, pos, symbols, star_pc, line),
    }
}

// primary = number | '$'hex | '%'bin | ''char'' | '*' | ident
//         | '(' expr ')' | 'hi' '(' expr ')' | 'lo' '(' expr ')'
fn parse_primary(
    toks: &[Token],
    pos: usize,
    symbols: &HashMap<String, u16>,
    star_pc: u16,
    line: usize,
) -> Result<(u16, usize), AssemblerError> {
    if pos >= toks.len() {
        return Err(AssemblerError::UnexpectedEnd(line));
    }
    match toks[pos] {
        Token::Number(n) => Ok((n, pos + 1)),
        Token::Star => Ok((star_pc, pos + 1)),
        Token::LParen => {
            let (v, p) = parse_expr(toks, pos + 1, symbols, star_pc, line)?;
            if p >= toks.len() || toks[p] != Token::RParen {
                return Err(AssemblerError::ExpressionError("expected ')'".into(), line));
            }
            Ok((v, p + 1))
        }
        Token::Hi => {
            // Support both `hi(expr)` and `hi expr` (space-separated).
            if pos + 1 < toks.len() && toks[pos + 1] == Token::LParen {
                let (v, p) = parse_expr(toks, pos + 2, symbols, star_pc, line)?;
                if p >= toks.len() || toks[p] != Token::RParen {
                    return Err(AssemblerError::ExpressionError("expected ')' after hi()".into(), line));
                }
                Ok(((v >> 8) & 0xFF, p + 1))
            } else {
                let (v, p) = parse_expr(toks, pos + 1, symbols, star_pc, line)?;
                Ok(((v >> 8) & 0xFF, p))
            }
        }
        Token::Lo => {
            // Support both `lo(expr)` and `lo expr` (space-separated).
            if pos + 1 < toks.len() && toks[pos + 1] == Token::LParen {
                let (v, p) = parse_expr(toks, pos + 2, symbols, star_pc, line)?;
                if p >= toks.len() || toks[p] != Token::RParen {
                    return Err(AssemblerError::ExpressionError("expected ')' after lo()".into(), line));
                }
                Ok((v & 0xFF, p + 1))
            } else {
                let (v, p) = parse_expr(toks, pos + 1, symbols, star_pc, line)?;
                Ok((v & 0xFF, p))
            }
        }
        Token::Ident(name) => {
            let key = name.to_ascii_lowercase();
            match symbols.get(&key) {
                Some(&v) => Ok((v, pos + 1)),
                None => Err(AssemblerError::UnknownLabel(name.to_string(), line)),
            }
        }
        _ => Err(AssemblerError::ExpressionError(
            format!("unexpected token {:?}", toks[pos]),
            line,
        )),
    }
}

/// Evaluate an expression string, returning the numeric value.
fn eval_expr(expr: &str, symbols: &HashMap<String, u16>, star_pc: u16, line: usize) -> Result<u16, AssemblerError> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return Err(AssemblerError::ExpressionError("empty expression".into(), line));
    }
    let tokens = tokenize_expr(trimmed, line)?;
    let (val, pos) = parse_expr(&tokens, 0, symbols, star_pc, line)?;
    if pos < tokens.len() && tokens[pos] != Token::Comma && tokens[pos] != Token::Eof {
        return Err(AssemblerError::ExpressionError(
            format!("trailing tokens in `{trimmed}`"),
            line,
        ));
    }
    Ok(val)
}

/// Evaluate an `if` condition, treating unknown labels as 0 (false-y).
fn eval_cond_expr(expr: &str, symbols: &HashMap<String, u16>, line: usize) -> u16 {
    eval_expr(expr, symbols, 0, line).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Macro Expander — text-level pre-processor
// ---------------------------------------------------------------------------

/// A collected macro definition (body lines).
#[derive(Clone)]
struct MacroDef {
    body: Vec<String>,
}

/// Expands macros at the text level: evaluates `if`/`else`/`endif` conditionals,
/// collects macro bodies from active branches, and expands macro invocations.
///
/// This runs BEFORE `preprocess()` in the assembly pipeline.
fn expand_source(source: &str) -> Result<String, AssemblerError> {
    let mut expander = MacroExpander::new();
    expander.process(source)
}

struct MacroExpander {
    symbols: HashMap<String, u16>,
    macros: HashMap<String, MacroDef>,
    if_stack: Vec<bool>,
    output: Vec<String>,
    collecting: Option<String>, // name of macro being collected
    body_buf: Vec<String>,      // body lines being collected
    unique_counter: u32,
    /// Depth of `if`/`else`/`endif` nesting inside the macro body being collected.
    /// Non-zero means the collected body lines are inside an if-block.
    collecting_if_depth: usize,
}

impl MacroExpander {
    fn new() -> Self {
        let mut symbols = HashMap::new();
        // Pre-populate common status-flag constants (same as Assembler::new).
        symbols.insert("carry".into(), 0x01);
        symbols.insert("zero".into(), 0x02);
        symbols.insert("intdis".into(), 0x04);
        symbols.insert("decmode".into(), 0x08);
        symbols.insert("break".into(), 0x10);
        symbols.insert("reserv".into(), 0x20);
        symbols.insert("overfl".into(), 0x40);
        symbols.insert("minus".into(), 0x80);

        Self {
            symbols,
            macros: HashMap::new(),
            if_stack: Vec::new(),
            output: Vec::new(),
            collecting: None,
            body_buf: Vec::new(),
            unique_counter: 0,
            collecting_if_depth: 0,
        }
    }

    /// True when we are not inside a skipped `if`/`else`/`endif` block.
    fn is_active(&self) -> bool {
        self.if_stack.iter().all(|&active| active)
    }

    /// Process source text line-by-line, expanding macros.
    fn process(&mut self, source: &str) -> Result<String, AssemblerError> {
        // Phase 1: Extract all `name = expr` / `name equ expr` definitions.
        // We need this before processing conditionals so symbols like
        // `report`, `I_flag`, `disable_decimal`, etc. are available.
        self.extract_equ_definitions(source);

        // Phase 2: Main line-by-line processing.
        for (idx, raw) in source.lines().enumerate() {
            let line_num = idx + 1;
            // Strip comments before any processing.
            let clean = if let Some(sc) = raw.find(';') { &raw[..sc] } else { raw };
            let trimmed = clean.trim();
            self.process_line(trimmed, line_num)?;
        }

        // Phase 3: If still collecting a macro body that was never finished,
        // discard it (no endm seen — treat as error, but for now discard).
        self.collecting = None;
        self.body_buf.clear();
        self.collecting_if_depth = 0;

        Ok(self.output.join("\n"))
    }

    /// Scan source for `name = expr` / `name equ expr` to pre-populate symbols.
    fn extract_equ_definitions(&mut self, source: &str) {
        for line in source.lines() {
            let clean = if let Some(sc) = line.find(';') {
                &line[..sc]
            } else {
                line
            };
            let trimmed = clean.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Try to match a simple `name = expr` or `name equ expr` pattern.
            let lower = trimmed.to_ascii_lowercase();
            if let Some(_eq_pos) = lower.find(|c: char| c == '=' || c == 'e') {
                let (name, rest) = if lower.contains(" equ ") || lower.starts_with("equ ") {
                    // `name equ expr` or `equ name` — skip the latter.
                    let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
                    if parts.len() < 2 {
                        continue;
                    }
                    // `equ` is the first word → skip (not an eq-def)
                    if parts[0].eq_ignore_ascii_case("equ") {
                        continue;
                    }
                    // Split: name, "equ expr"
                    let rest = parts[1].trim();
                    if !rest.to_ascii_lowercase().starts_with("equ ") && rest != "equ" {
                        continue;
                    }
                    let expr = rest.trim_start_matches("equ ").trim_start_matches("equ").trim();
                    if expr.is_empty() {
                        continue;
                    }
                    (parts[0], expr)
                } else if let Some(eq) = lower.find('=') {
                    let name = trimmed[..eq].trim();
                    if name.is_empty() || name.contains(' ') || name.contains('\t') {
                        continue;
                    }
                    // Skip conditionals and directives.
                    let name_lower = name.to_ascii_lowercase();
                    if name_lower == "if"
                        || name_lower == "else"
                        || name_lower == "endif"
                        || name_lower == "macro"
                        || name_lower == "endm"
                    {
                        continue;
                    }
                    let expr = trimmed[eq + 1..].trim();
                    if expr.is_empty() {
                        continue;
                    }
                    (name, expr)
                } else {
                    continue;
                };

                // Skip names with special chars / not simple identifiers.
                if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                // Skip reserved words.
                let name_lower = name.to_ascii_lowercase();
                if is_directive(&name_lower) || parse_mnemonic(&name_lower).is_some() {
                    continue;
                }

                // Evaluate the expression and store.
                if let Ok(val) = eval_expr(rest, &self.symbols, 0, 0) {
                    self.symbols.insert(name_lower, val);
                }
            }
        }
    }

    fn process_line(&mut self, line: &str, line_num: usize) -> Result<(), AssemblerError> {
        if line.is_empty() {
            if self.collecting.is_none() {
                self.output.push(String::new());
            } else {
                self.body_buf.push(String::new());
            }
            return Ok(());
        }

        let lower = line.to_ascii_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();
        let first = words.first().copied().unwrap_or("");

        // --- Macro body collection ---
        if let Some(ref macro_name) = self.collecting.clone() {
            if first == "endm" || first == "endmacro" {
                // Check if we have any open if-inside-macro.
                if self.collecting_if_depth == 0 {
                    // Store the macro definition.
                    let body = std::mem::take(&mut self.body_buf);
                    self.macros.insert(macro_name.clone(), MacroDef { body });
                    self.collecting = None;
                } else {
                    // End of a nested if-block inside the macro body.
                    self.collecting_if_depth -= 1;
                    self.body_buf.push(line.to_string());
                }
            } else {
                self.body_buf.push(line.to_string());
            }
            return Ok(());
        }

        // --- Top-level processing (not collecting a macro) ---

        // Conditionals are evaluated at the top level.
        if first == "if" {
            let cond = words[1..].join(" ");
            let val = eval_cond_expr(&cond, &self.symbols, line_num);
            self.if_stack.push(val != 0);
            return Ok(());
        }
        if first == "else" {
            if let Some(top) = self.if_stack.last_mut() {
                *top = !*top;
            }
            return Ok(());
        }
        if first == "endif" {
            self.if_stack.pop();
            return Ok(());
        }

        // Skip lines in inactive branches.
        if !self.is_active() {
            return Ok(());
        }

        // --- Macro definition ---
        if words.len() >= 2 && words[1] == "macro" {
            let name = words[0].to_string();
            self.collecting = Some(name);
            self.body_buf.clear();
            self.collecting_if_depth = 0;
            return Ok(());
        }

        // --- Macro invocation ---
        if let Some(mdef) = self.macros.get(first) {
            // Clone the body to avoid borrow conflict with &mut self.
            let body = mdef.body.clone();
            // Extract invocation arguments.
            let args_str = line
                .trim_start()
                .split_whitespace()
                .skip(1)
                .collect::<Vec<_>>()
                .join(" ");
            let args: Vec<&str> = if args_str.is_empty() {
                vec![]
            } else {
                args_str.split(',').map(|a| a.trim()).collect()
            };

            let expanded = self.expand_macro_body(&body, &args, line_num)?;
            for expanded_line in &expanded {
                // Recursively process the expanded line (may contain further
                // macro invocations or conditionals).
                self.process_line(expanded_line, line_num)?;
            }
            return Ok(());
        }

        // --- Regular line — pass through ---
        self.output.push(line.to_string());
        Ok(())
    }

    /// Expand a macro body by substituting `\N` params and `\?` unique labels,
    /// then returning the expanded lines.
    fn expand_macro_body(
        &mut self,
        body: &[String],
        args: &[&str],
        _line: usize,
    ) -> Result<Vec<String>, AssemblerError> {
        self.unique_counter += 1;
        let uniq_id = format!("?{:04X}", self.unique_counter);

        let mut expanded = Vec::new();
        for line in body {
            let mut result = line.clone();
            // Replace \? with unique label id.
            result = result.replace("\\?", &uniq_id);
            // Replace \1, \2, ... \N with positional arguments.
            for (i, arg) in args.iter().enumerate() {
                let param = format!("\\{}", i + 1);
                result = result.replace(&param, arg);
            }
            expanded.push(result);
        }
        Ok(expanded)
    }
}

// ---------------------------------------------------------------------------
// Line-level parsing
// ---------------------------------------------------------------------------

/// Pre-process source into parsed lines (strip comments, classify).
fn preprocess(source: &str) -> Vec<ParsedLine> {
    // Macros have already been expanded by `expand_source()`, so this function
    // simply parses lines (strip comments, classify). No macro handling needed.
    source
        .lines()
        .enumerate()
        .filter_map(|(idx, raw)| {
            let line_num = idx + 1;
            // Strip comments
            let trimmed = if let Some(sc) = raw.find(';') { &raw[..sc] } else { raw };
            let trimmed = trimmed.trim();
            if trimmed.is_empty() {
                return Some(ParsedLine {
                    label: None,
                    kind: LineKind::Skip,
                    line: line_num,
                    operand: String::new(),
                });
            }
            parse_line(trimmed, line_num)
        })
        .collect()
}

/// Parse a single non-empty, non-comment line.
fn parse_line(line: &str, line_num: usize) -> Option<ParsedLine> {
    let (label, rest) = extract_label(line)?;
    let rest = rest.trim();
    if rest.is_empty() {
        // Label on its own line — still emit it as Skip; the label is registered.
        return Some(ParsedLine {
            label,
            kind: LineKind::Skip,
            line: line_num,
            operand: String::new(),
        });
    }

    let (keyword, operand) = split_word(rest);
    let keyword = keyword.to_ascii_lowercase();

    let kind = match keyword.as_str() {
        "org" => LineKind::Org,
        "equ" | "=" => LineKind::Equ,
        "ds" | ".ds" => LineKind::Ds,
        "db" | ".db" | "byte" | ".byte" => LineKind::Db,
        "dw" | ".dw" | "word" | ".word" => LineKind::Dw,
        "end" => LineKind::End,
        "if" => LineKind::If(operand.to_string()),
        "else" => LineKind::Else,
        "endif" => LineKind::Endif,
        // Assembler directives we skip
        "code" | "data" | "bss" | "page" | "noopt" | "list" | "nolist" | "macro" | "endm" | "include" | "error" => {
            LineKind::Skip
        }
        mnem => {
            if let Some(m) = parse_mnemonic(mnem) {
                LineKind::Instruction(m)
            } else {
                // Unknown keyword — skip silently.
                LineKind::Skip
            }
        }
    };

    Some(ParsedLine {
        label,
        kind,
        line: line_num,
        operand: operand.to_string(),
    })
}

/// Extract a label from the start of a line.  Labels can be `name` or `name:`.
/// Returns `None` (and the original line) when no label is present.
fn extract_label(line: &str) -> Option<(Option<String>, &str)> {
    // Allow '?' in labels (generated by macro \? unique label expansion).
    let ident_len = line
        .chars()
        .take_while(|&c| c.is_alphanumeric() || c == '_' || c == '?')
        .count();
    if ident_len == 0 {
        return Some((None, line));
    }

    let (ident, after) = line.split_at(ident_len);

    // Label with colon: `name: ...`
    if let Some(stripped) = after.strip_prefix(':') {
        return Some((Some(ident.to_string()), stripped));
    }

    // `name = ...` — treat the name as a label (equate).
    if after.trim_start().starts_with('=') {
        return Some((Some(ident.to_string()), after));
    }

    // If followed by whitespace (or end), check if the word is a
    // mnemonic/directive.  If not, it's a bare label.
    if after.starts_with(char::is_whitespace) || after.is_empty() {
        let lower = ident.to_ascii_lowercase();
        if !is_reserved(&lower) {
            return Some((Some(ident.to_string()), after.trim_start()));
        }
    }

    // Not a label — return the line unmodified.
    Some((None, line))
}

/// True if the word is a reserved keyword (directive or mnemonic).
fn is_reserved(s: &str) -> bool {
    is_directive(s) || parse_mnemonic(s).is_some()
}

fn is_directive(s: &str) -> bool {
    matches!(
        s,
        "org"
            | "equ"
            | "="
            | "ds"
            | ".ds"
            | "db"
            | ".db"
            | "byte"
            | ".byte"
            | "dw"
            | ".dw"
            | "word"
            | ".word"
            | "end"
            | "code"
            | "data"
            | "bss"
            | "page"
            | "noopt"
            | "list"
            | "nolist"
            | "macro"
            | "endm"
            | "if"
            | "else"
            | "endif"
            | "include"
    )
}

/// Split a line into the first whitespace-delimited word and the remainder.
fn split_word(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    if let Some(pos) = s.find(|c: char| c.is_whitespace()) {
        let (kw, rest) = s.split_at(pos);
        (kw, rest.trim_start())
    } else {
        (s, "")
    }
}

/// Parse a mnemonic string (case-insensitive) into the Mnemonic enum.
fn parse_mnemonic(s: &str) -> Option<Mnemonic> {
    use Mnemonic::*;
    Some(match s {
        "adc" => Adc,
        "and" => And,
        "asl" => Asl,
        "bcc" => Bcc,
        "bcs" => Bcs,
        "beq" => Beq,
        "bit" => Bit,
        "bmi" => Bmi,
        "bne" => Bne,
        "bpl" => Bpl,
        "brk" => Brk,
        "bvc" => Bvc,
        "bvs" => Bvs,
        "clc" => Clc,
        "cld" => Cld,
        "cli" => Cli,
        "clv" => Clv,
        "cmp" => Cmp,
        "cpx" => Cpx,
        "cpy" => Cpy,
        "dec" => Dec,
        "dex" => Dex,
        "dey" => Dey,
        "eor" => Eor,
        "inc" => Inc,
        "inx" => Inx,
        "iny" => Iny,
        "jmp" => Jmp,
        "jsr" => Jsr,
        "lda" => Lda,
        "ldx" => Ldx,
        "ldy" => Ldy,
        "lsr" => Lsr,
        "nop" => Nop,
        "ora" => Ora,
        "pha" => Pha,
        "php" => Php,
        "pla" => Pla,
        "plp" => Plp,
        "rol" => Rol,
        "ror" => Ror,
        "rti" => Rti,
        "rts" => Rts,
        "sbc" => Sbc,
        "sec" => Sec,
        "sed" => Sed,
        "sei" => Sei,
        "sta" => Sta,
        "stx" => Stx,
        "sty" => Sty,
        "tax" => Tax,
        "tay" => Tay,
        "tsx" => Tsx,
        "txa" => Txa,
        "txs" => Txs,
        "tya" => Tya,
        // Known illegal opcodes
        "dcp" => Dcp,
        "isc" => Isc,
        "lax" => Lax,
        "rla" => Rla,
        "rra" => Rra,
        "sax" => Sax,
        "slo" => Slo,
        "sre" => Sre,
        "alr" => Alr,
        "anc" => Anc,
        "arr" => Arr,
        _ => return None,
    })
}

/// True if the mnemonic is a branch instruction.
fn is_branch(m: Mnemonic) -> bool {
    matches!(
        m,
        Mnemonic::Bcc
            | Mnemonic::Bcs
            | Mnemonic::Beq
            | Mnemonic::Bmi
            | Mnemonic::Bne
            | Mnemonic::Bpl
            | Mnemonic::Bvc
            | Mnemonic::Bvs
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Byte size of an addressing mode.
fn mode_size(mode: AddressingMode) -> u8 {
    match mode {
        AddressingMode::Implied | AddressingMode::Accumulator => 1,
        AddressingMode::Immediate
        | AddressingMode::ZeroPage
        | AddressingMode::ZeroPageX
        | AddressingMode::ZeroPageY
        | AddressingMode::Relative
        | AddressingMode::IndexedIndirect
        | AddressingMode::IndirectIndexed => 2,
        AddressingMode::Absolute | AddressingMode::AbsoluteX | AddressingMode::AbsoluteY | AddressingMode::Indirect => {
            3
        }
    }
}

/// Compute the byte size of a parsed line (for label offset calculation).
fn line_size(line: &ParsedLine, asm: &Assembler) -> Option<u16> {
    match &line.kind {
        LineKind::Org
        | LineKind::Equ
        | LineKind::End
        | LineKind::Skip
        | LineKind::If(_)
        | LineKind::Else
        | LineKind::Endif => Some(0),
        LineKind::Ds => asm.eval(&line.operand, line.line).ok(),
        LineKind::Db => Some(split_values(&line.operand).len() as u16),
        LineKind::Dw => Some((split_values(&line.operand).len() * 2) as u16),
        LineKind::Instruction(m) => {
            // Use emit=true so that known zero-page addresses are correctly
            // sized (2 bytes) instead of falling back to Absolute (3 bytes).
            // This keeps label registration in pass1 consistent with pass1's
            // own mode detection (which also uses emit=true).
            let mode = asm.detect_mode(&line.operand, *m, line.line, true).ok()?;
            Some(mode_size(mode) as u16)
        }
    }
}

/// Split a comma-separated value list, trimming whitespace.
fn split_values(s: &str) -> Vec<&str> {
    s.split(',').map(|p| p.trim()).filter(|p| !p.is_empty()).collect()
}

/// Build the (Mnemonic, AddressingMode) → opcode-byte map from the OPCODES table.
fn build_opcode_map() -> HashMap<(Mnemonic, AddressingMode), u8> {
    use crate::opcode::OPCODES;
    let mut map = HashMap::new();
    for (opcode, info) in OPCODES.iter().enumerate() {
        if info.mnemonic != Mnemonic::Illegal {
            let key = (info.mnemonic, info.mode);
            // First entry wins (the canonical/standard encoding)
            map.entry(key).or_insert(opcode as u8);
        }
    }
    map
}

/// True if a (mnemonic, addressing-mode) combo has an encoding in the opcode table.
fn opcode_exists(mnemonic: Mnemonic, mode: AddressingMode) -> bool {
    OPCODES
        .iter()
        .any(|info| info.mnemonic == mnemonic && info.mode == mode)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn assemble_str(src: &str) -> Vec<u8> {
        assemble(src, 0x1000, None).unwrap().0
    }

    #[test]
    fn test_empty() {
        assert!(assemble("", 0x1000, None).unwrap().0.is_empty());
    }

    #[test]
    fn test_only_comments() {
        assert!(assemble_str("; just comments\n; another\n").is_empty());
    }

    #[test]
    fn test_org() {
        // org matching default origin → no padding
        assert!(assemble_str("org $1000").is_empty());
    }

    #[test]
    fn test_lda_imm() {
        assert_eq!(assemble_str("org $1000\nlda #$42\nbrk"), vec![0xA9, 0x42, 0x00]);
    }

    #[test]
    fn test_zero_page_vs_absolute() {
        let bytes = assemble_str("org $1000\nlda $20\nsta $2000\n");
        // lda $20 = A5 20, sta $2000 = 8D 00 20
        assert_eq!(bytes, vec![0xA5, 0x20, 0x8D, 0x00, 0x20]);
    }

    #[test]
    fn test_indexed() {
        let bytes = assemble_str("org $1000\nlda $40,X\nsta $2000,X\nldx $50,Y\n");
        // lda $40,X = B5 40, sta $2000,X = 9D 00 20, ldx $50,Y = B6 50
        assert_eq!(bytes, vec![0xB5, 0x40, 0x9D, 0x00, 0x20, 0xB6, 0x50]);
    }

    #[test]
    fn test_indirect() {
        let bytes = assemble_str("org $1000\nlda ($40,X)\nlda ($40),Y\njmp ($2000)\n");
        assert_eq!(bytes, vec![0xA1, 0x40, 0xB1, 0x40, 0x6C, 0x00, 0x20]);
    }

    #[test]
    fn test_branch_backward() {
        let bytes = assemble_str("org $1000\nloop dex\nbne loop\n");
        // dex = CA at $1000
        // bne loop at $1001: target=$1000, next=$1003, offset=$FD
        assert_eq!(bytes, vec![0xCA, 0xD0, 0xFD]);
    }

    #[test]
    fn test_branch_forward() {
        let bytes = assemble_str("org $1000\nbeq skip\nbrk\nskip nop\n");
        // beq at $1000: target=$1003, next=$1002, offset=1
        assert_eq!(bytes, vec![0xF0, 0x01, 0x00, 0xEA]);
    }

    #[test]
    fn test_db_dw_ds() {
        let bytes = assemble_str("org $1000\ndb $01, $02, $03\ndw $abcd\nds 4\n");
        assert_eq!(bytes, vec![0x01, 0x02, 0x03, 0xCD, 0xAB, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_equ() {
        let (bytes, syms) = assemble("org $1000\nfoo = $42\nlda #foo\n", 0x1000, None).unwrap();
        assert_eq!(bytes, vec![0xA9, 0x42]);
        assert_eq!(syms.get("foo"), Some(&0x42));
    }

    #[test]
    fn test_label_ref() {
        let bytes = assemble_str("org $1000\nstart lda #0\nsta val\nbrk\nval ds 1\n");
        // lda #0: A9 00 at $1000
        // sta val at $1002: val=$1006 → 8D 06 10
        // brk: 00 at $1005
        // ds 1: 00 at $1006
        assert_eq!(bytes, vec![0xA9, 0x00, 0x8D, 0x06, 0x10, 0x00, 0x00]);
    }

    #[test]
    fn test_illegal_immediate() {
        let bytes = assemble_str("org $1000\nanc #$42\nalr #$42\narr #$42\n");
        // anc = 0x0B, alr = 0x4B, arr = 0x6B
        assert_eq!(bytes, vec![0x0B, 0x42, 0x4B, 0x42, 0x6B, 0x42]);
    }

    #[test]
    fn test_illegal_rmw() {
        let bytes = assemble_str("org $1000\ndcp $20\nisc $2000\nrla $40,X\nslo $50\nsre $60\n");
        // dcp zp 0xC7, isc abs 0xEF, rla zp,x 0x37, slo zp 0x07, sre zp 0x47
        assert_eq!(
            bytes,
            vec![0xC7, 0x20, 0xEF, 0x00, 0x20, 0x37, 0x40, 0x07, 0x50, 0x47, 0x60]
        );
    }

    #[test]
    fn test_expressions() {
        let bytes = assemble_str("org $1000\nlda #$20+$22\nldx #$40|3\nldy #$FF&$0F\nsta $20+$10\n");
        // A9 42, A2 43, A0 0F, 85 30
        assert_eq!(bytes, vec![0xA9, 0x42, 0xA2, 0x43, 0xA0, 0x0F, 0x85, 0x30]);
    }

    #[test]
    fn test_hi_lo() {
        let bytes = assemble_str("org $1000\nlda #hi($1234)\nlda #lo($1234)\n");
        assert_eq!(bytes, vec![0xA9, 0x12, 0xA9, 0x34]);
    }

    #[test]
    fn test_pc_star() {
        let bytes = assemble_str("org $1000\nbeq *+$10\n");
        // beq at $1000: next=$1002, target=$1010, offset=$0E
        assert_eq!(bytes, vec![0xF0, 0x0E]);
    }

    #[test]
    fn test_predefined_symbols() {
        let mut syms = HashMap::new();
        syms.insert("code_segment".into(), 0x0400u16);
        let (bytes, _) = assemble("org code_segment\nlda #0\n", 0x0400, Some(syms)).unwrap();
        assert_eq!(bytes, vec![0xA9, 0x00]);
    }

    #[test]
    fn test_implied_all() {
        let src = "\
org $1000
clc
cld
cli
clv
dex
dey
inx
iny
nop
pha
php
pla
plp
rti
rts
sec
sed
sei
tax
tay
tsx
txa
txs
tya
brk";
        let bytes = assemble_str(src);
        // 25 implied opcodes
        assert_eq!(bytes.len(), 25);
        assert_eq!(bytes[0], 0x18); // clc
        assert_eq!(bytes[8], 0xEA); // nop
        assert_eq!(bytes[14], 0x60); // rts
        assert_eq!(bytes[24], 0x00); // brk
    }

    #[test]
    fn test_accumulator_mode() {
        let bytes = assemble_str("org $1000\nasl\nlsr\nrol\nror\n");
        assert_eq!(bytes, vec![0x0A, 0x4A, 0x2A, 0x6A]);
    }

    #[test]
    fn test_comprehensive() {
        let src = r#"
            org $1000
        start lda #$42
            sta $2000
            ldx #$05
        loop dex
            bne loop
            lda $30
            ldy #$03
        l1  lda ($40),Y
            dey
            bne l1
            lda result
            brk

            db $01, $02, $03
            dw $abcd
        result ds 2
        "#;
        let (bytes, syms) = assemble(src, 0x1000, None).unwrap();

        let expected = vec![
            0xA9, 0x42, // lda #$42
            0x8D, 0x00, 0x20, // sta $2000
            0xA2, 0x05, // ldx #$05
            0xCA, // dex (loop=$1007)
            0xD0, 0xFD, // bne loop (target $1007, next $100A)
            0xA5, 0x30, // lda $30
            0xA0, 0x03, // ldy #$03
            0xB1, 0x40, // lda ($40),Y (l1=$100E)
            0x88, // dey
            0xD0, 0xFB, // bne l1 (target $100E, next $1013)
            0xAD, 0x1C, 0x10, // lda result (result=$101C)
            0x00, // brk
            0x01, 0x02, 0x03, // db
            0xCD, 0xAB, // dw $abcd
            0x00, 0x00, // ds 2
        ];
        assert_eq!(bytes, expected);
        assert_eq!(syms.get("start"), Some(&0x1000));
        assert_eq!(syms.get("loop"), Some(&0x1007));
        assert_eq!(syms.get("l1"), Some(&0x100E));
        assert_eq!(syms.get("result"), Some(&0x101C));
    }

    #[test]
    fn test_undefined_label() {
        let err = assemble("org $1000\nlda unknown\n", 0, None).unwrap_err();
        assert!(err.to_string().contains("unknown label"), "{err}");
    }

    #[test]
    fn test_duplicate_label() {
        let err = assemble("org $1000\nfoo nop\nfoo nop\n", 0, None).unwrap_err();
        assert!(err.to_string().contains("duplicate label"), "{err}");
    }

    #[test]
    fn test_jsr_rts() {
        let bytes = assemble_str("org $1000\njsr $2000\nrts\n");
        assert_eq!(bytes, vec![0x20, 0x00, 0x20, 0x60]);
    }

    #[test]
    fn test_jmp_abs_indirect() {
        let bytes = assemble_str("org $1000\njmp $3000\njmp ($4000)\n");
        assert_eq!(bytes, vec![0x4C, 0x00, 0x30, 0x6C, 0x00, 0x40]);
    }

    #[test]
    fn test_bit_zp_abs() {
        let bytes = assemble_str("org $1000\nbit $20\nbit $2000\n");
        assert_eq!(bytes, vec![0x24, 0x20, 0x2C, 0x00, 0x20]);
    }

    #[test]
    fn test_stx_sty() {
        let bytes = assemble_str("org $1000\nsty $20\nstx $30\nsty $2000\nstx $3000\n");
        // sty zp = 84, stx zp = 86, sty abs = 8C, stx abs = 8E
        assert_eq!(bytes, vec![0x84, 0x20, 0x86, 0x30, 0x8C, 0x00, 0x20, 0x8E, 0x00, 0x30]);
    }

    #[test]
    fn test_stx_zpy() {
        let bytes = assemble_str("org $1000\nstx $10,Y\n");
        // stx $10,Y = 96 10  (ZeroPageY)
        assert_eq!(bytes, vec![0x96, 0x10]);
    }

    #[test]
    fn test_cpx_cpy() {
        let bytes = assemble_str("org $1000\ncpx #$10\ncpy #$20\ncpx $30\ncpy $40\ncpx $2000\ncpy $3000\n");
        // cpx imm = E0, cpy imm = C0
        // cpx zp = E4, cpy zp = C4
        // cpx abs = EC, cpy abs = CC
        assert_eq!(
            bytes,
            vec![0xE0, 0x10, 0xC0, 0x20, 0xE4, 0x30, 0xC4, 0x40, 0xEC, 0x00, 0x20, 0xCC, 0x00, 0x30],
        );
    }

    #[test]
    fn test_ldx_zp_abs() {
        let bytes = assemble_str("org $1000\nldx $20\nldx $2000\nldx $2000,Y\n");
        // ldx zp = A6, ldx abs = AE, ldx abs,Y = BE
        // ldx $20 = A6 20, ldx $2000 = AE 00 20, ldx $2000,Y = BE 00 20
        assert_eq!(bytes, vec![0xA6, 0x20, 0xAE, 0x00, 0x20, 0xBE, 0x00, 0x20]);
    }

    #[test]
    fn test_ldy_zp_abs() {
        let bytes = assemble_str("org $1000\nldy $20\nldy $2000\nldy $2000,X\n");
        // ldy zp = A4, ldy abs = AC, ldy abs,X = BC
        assert_eq!(bytes, vec![0xA4, 0x20, 0xAC, 0x00, 0x20, 0xBC, 0x00, 0x20]);
    }

    #[test]
    fn test_inc_dec() {
        let bytes = assemble_str("org $1000\ninc $20\ndec $30\ninc $2000\ndec $3000\ninc $40,X\ndec $50,X\n");
        // inc zp = E6, dec zp = C6
        // inc abs = EE, dec abs = CE
        // inc zp,x = F6, dec zp,x = D6
        assert_eq!(
            bytes,
            vec![0xE6, 0x20, 0xC6, 0x30, 0xEE, 0x00, 0x20, 0xCE, 0x00, 0x30, 0xF6, 0x40, 0xD6, 0x50,],
        );
    }

    #[test]
    fn test_binary_literal() {
        let bytes = assemble_str("org $1000\nlda #%10100101\n");
        // %10100101 = $A5
        assert_eq!(bytes, vec![0xA9, 0xA5]);
    }

    #[test]
    fn test_char_literal() {
        let bytes = assemble_str("org $1000\nlda #'A'\n");
        assert_eq!(bytes, vec![0xA9, 0x41]);
    }

    #[test]
    fn test_parentheses_expr() {
        let bytes = assemble_str("org $1000\nlda #((2+3)*$10)\n");
        // (2+3)*16 = 5*16 = 80 = $50
        assert_eq!(bytes, vec![0xA9, 0x50]);
    }

    #[test]
    fn test_unary_minus() {
        let bytes = assemble_str("org $1000\nlda #-$01\n");
        // -1 in two's complement 8-bit = $FF
        assert_eq!(bytes, vec![0xA9, 0xFF]);
    }

    #[test]
    fn test_bitwise_not() {
        let bytes = assemble_str("org $1000\nlda #~$0F\n");
        // ~$0F = $FFF0, truncated to 8-bit = $F0
        assert_eq!(bytes, vec![0xA9, 0xF0]);
    }

    #[test]
    fn test_macro_expansion() {
        // Macros should be expanded — the body replaces the invocation.
        let src = r#"
            org $1000
            trap macro
                jmp *
                endm
            nop
            trap
        "#;
        let bytes = assemble_str(src);
        // nop=EA at $1000, then macro body expanded: jmp * at $1001 = 4C 01 10
        assert_eq!(bytes, vec![0xEA, 0x4C, 0x01, 0x10]);
    }

    #[test]
    fn test_macro_with_params() {
        // Macro with parameters: \1, \2 substitution.
        let src = r#"
            org $1000
            set_a macro
                lda #\1
                ldx #\2
                endm
            set_a $42, $99
        "#;
        let bytes = assemble_str(src);
        // lda #$42 = A9 42, ldx #$99 = A2 99
        assert_eq!(bytes, vec![0xA9, 0x42, 0xA2, 0x99]);
    }

    #[test]
    fn test_macro_unique_labels() {
        // \? generates unique labels so multiple invocations don't collide.
        let src = r#"
            org $1000
            mytrap macro
                bne skip\?
                brk
            skip\?
                endm
            mytrap
            mytrap
        "#;
        let bytes = assemble_str(src);
        // First invocation: bne $1003 (skip?0001), brk=00
        // Second invocation: bne $1006 (skip?0002), brk=00
        // Total: 2+1 + 2+1 = 6 bytes
        assert_eq!(bytes.len(), 6);
        assert_eq!(bytes[0], 0xD0); // bne
        assert_eq!(bytes[1], 0x01); // offset to skip first brk
        assert_eq!(bytes[2], 0x00); // brk
        assert_eq!(bytes[3], 0xD0); // bne
        assert_eq!(bytes[4], 0x01); // offset to skip second brk
        assert_eq!(bytes[5], 0x00); // brk
    }

    #[test]
    fn test_macro_nested_invocation() {
        // A macro body can invoke another macro.
        let src = r#"
            org $1000
            load_flag macro
                lda #\1
                endm
            set_stat macro
                load_flag \1
                pha
                plp
                endm
            set_stat $42
        "#;
        let bytes = assemble_str(src);
        // load_flag $42 expands to: lda #$42 = A9 42
        // then pha = 48, plp = 28
        assert_eq!(bytes, vec![0xA9, 0x42, 0x48, 0x28]);
    }

    #[test]
    fn test_macro_if_in_body() {
        // Conditionals inside a macro body are evaluated at expansion time.
        let src = r#"
            org $1000
            myop macro
                if \1
                nop
                endif
                endm
            myop 0
            myop 1
        "#;
        let bytes = assemble_str(src);
        // First invocation: if 0 → skip nop
        // Second invocation: if 1 → nop = EA
        assert_eq!(bytes, vec![0xEA]);
    }

    #[test]
    fn test_if_true() {
        // `if 1` is always true, so the nop is assembled.
        let bytes = assemble_str("org $1000\nif 1\nnop\nendif\n");
        assert_eq!(bytes, vec![0xEA]);
    }

    #[test]
    fn test_if_false() {
        // `if 0` is always false, so the nop is skipped.
        let bytes = assemble_str("org $1000\nif 0\nnop\nendif\n");
        assert_eq!(bytes, vec![]);
    }

    #[test]
    fn test_if_else() {
        // `if 0` → else branch taken.
        let src = r#"
            org $1000
            if 0
            nop
            else
            inx
            endif
        "#;
        let bytes = assemble_str(src);
        assert_eq!(bytes, vec![0xE8]); // inx
    }

    #[test]
    fn test_if_unknown_symbol_false() {
        // Unknown symbol in condition → eval_cond returns 0 → false.
        let bytes = assemble_str("org $1000\nif disable_decimal\ndex\nendif\n");
        // Condition is false, so dex is skipped.
        assert_eq!(bytes, vec![]);
    }

    #[test]
    fn test_if_known_label_true() {
        // A known label resolves to its value; non-zero → true.
        let src = r#"
            org $1000
            debug = 1
            if debug
            inx
            endif
        "#;
        let bytes = assemble_str(src);
        assert_eq!(bytes, vec![0xE8]);
    }

    #[test]
    fn test_flag_constants() {
        // The predefined flag constants should be available
        let bytes = assemble_str("org $1000\nlda #carry\nlda #zero\nlda #intdis\nlda #minus\n");
        // carry=1, zero=2, intdis=4, minus=$80
        assert_eq!(bytes, vec![0xA9, 0x01, 0xA9, 0x02, 0xA9, 0x04, 0xA9, 0x80]);
    }

    #[test]
    fn test_org_padding() {
        let bytes = assemble_str("org $1000\ndb $AA\norg $1010\ndb $BB\n");
        // $1000: AA
        // $1001-100F: 00 padding
        // $1010: BB
        assert_eq!(bytes.len(), 0x11);
        assert_eq!(bytes[0], 0xAA);
        assert_eq!(bytes[0x10], 0xBB);
        // All bytes between should be zero
        assert!(bytes[1..0x10].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_lax_sax() {
        let bytes = assemble_str("org $1000\nlax $20\nsax $30\nlax $2000\nsax $3000\n");
        // lax zp = A7, sax zp = 87, lax abs = AF, sax abs = 8F
        assert_eq!(bytes, vec![0xA7, 0x20, 0x87, 0x30, 0xAF, 0x00, 0x20, 0x8F, 0x00, 0x30]);
    }

    #[test]
    fn test_end_stops() {
        let src = r#"
            org $1000
            lda #$01
            end
            lda #$02
        "#;
        let bytes = assemble_str(src);
        // Only the first lda should be assembled
        assert_eq!(bytes, vec![0xA9, 0x01]);
    }

    #[test]
    fn test_label_with_colon() {
        let src = "org $1000\nstart: nop\nloop: dex\nbne loop\n";
        let bytes = assemble_str(src);
        assert_eq!(bytes, vec![0xEA, 0xCA, 0xD0, 0xFD]);
    }

    #[test]
    fn test_ff_minus_zero_expr() {
        // Test how `$ff-zero` evaluates when `zero = %00000010`
        // Expected: $ff - $02 = $FD
        // Then: cmp #($fd|$30)&$ff = cmp #$fd
        let src = "org $1000\n\
                    zero = %00000010\n\
                    cmp #($ff-zero|$30)&$ff\n";
        // C9 FD = cmp #$FD
        // C9 CD = cmp #$CD (WRONG - would mean $ff-zero = $ff-($02|$30) = $ff-$32 = $CD)
        let bytes = assemble_str(src);
        assert_eq!(
            bytes,
            vec![0xC9, 0xFD],
            "expected C9 FD (cmp #$FD), got: {:02X?}",
            bytes
        );
    }

    #[test]
    fn test_cmp_flag_ff_zero() {
        // Simulate cmp_flag macro (I_flag=3): cmp #(($ff-zero)|fao)&m8
        // fao = $30, m8 = $ff
        let src = "org $1000\n\
                    zero = %00000010\n\
                    fao = $30\n\
                    cmp #(($ff-zero)|fao)&$ff\n";
        let bytes = assemble_str(src);
        assert_eq!(
            bytes,
            vec![0xC9, 0xFD],
            "expected C9 FD (cmp #$FD), got: {:02X?}",
            bytes
        );
    }
}
