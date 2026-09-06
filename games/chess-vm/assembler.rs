pub const CODE_LIMIT: usize = 256;

pub const DATA_LIMIT: usize = 1024;

pub const CYCLE_BUDGET: u64 = 20_000_000;

pub const REGISTERS: usize = 16;

const DATA_BASE: i32 = 0;

const NO_SEARCH: i32 = 1;

const COMMENT: char = ';';
const LABEL_END: char = ':';
const SEARCH_DIRECTIVE: &str = ".search";
const DATA_DIRECTIVE: &str = ".data";
const WORD_DIRECTIVE: &str = ".word";
const ZERO_DIRECTIVE: &str = ".zero";
const REGISTER_PREFIX: [char; 2] = ['r', 'R'];
const HEX_PREFIX: [&str; 2] = ["0x", "0X"];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Opcode {
    LoadImmediate,
    Move,
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Minimum,
    Maximum,
    AddImmediate,
    MultiplyImmediate,
    Negate,
    LoadData,
    Jump,
    JumpIfZero,
    JumpIfNotZero,
    JumpIfLess,
    JumpIfLessOrEqual,
    JumpIfGreater,
    JumpIfGreaterOrEqual,
    Score,
    Halt,
    Material,
    Gain,
    Mobility,
    OpponentMobility,
    GivesCheck,
    IsCapture,
    IsCastle,
    IsPromotion,
    Promotion,
    MateInOne,
    KingDistance,
    OwnKingDistance,
    EnemyKingDistance,
    Exposed,
    LightCount,
    Attackers,
    Defenders,
    FromSquare,
    ToSquare,
    ToRank,
    MovedPiece,
    Center,
    PieceAt,
    StaticExchange,
    Hangs,
    Safe,
    MoveCount,
    Ply,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Operands {
    WriteImmediate,
    WriteRead,
    WriteReadRead,
    WriteReadImmediate,
    Label,
    ReadLabel,
    ReadReadLabel,
    Read,
    Write,
    Nothing,
}

impl Operands {
    fn count(&self) -> usize {
        match self {
            Operands::WriteReadRead | Operands::WriteReadImmediate | Operands::ReadReadLabel => 3,
            Operands::WriteRead | Operands::WriteImmediate | Operands::ReadLabel => 2,
            Operands::Label | Operands::Read | Operands::Write => 1,
            Operands::Nothing => 0,
        }
    }
}

const MNEMONICS: [(&str, Opcode, Operands, u64); 50] = [
    ("ldi", Opcode::LoadImmediate, Operands::WriteImmediate, 1),
    ("mov", Opcode::Move, Operands::WriteRead, 1),
    ("add", Opcode::Add, Operands::WriteReadRead, 1),
    ("sub", Opcode::Subtract, Operands::WriteReadRead, 1),
    ("mul", Opcode::Multiply, Operands::WriteReadRead, 1),
    ("div", Opcode::Divide, Operands::WriteReadRead, 1),
    ("mod", Opcode::Modulo, Operands::WriteReadRead, 1),
    ("min", Opcode::Minimum, Operands::WriteReadRead, 1),
    ("max", Opcode::Maximum, Operands::WriteReadRead, 1),
    (
        "addi",
        Opcode::AddImmediate,
        Operands::WriteReadImmediate,
        1,
    ),
    (
        "muli",
        Opcode::MultiplyImmediate,
        Operands::WriteReadImmediate,
        1,
    ),
    ("neg", Opcode::Negate, Operands::WriteRead, 1),
    ("ld", Opcode::LoadData, Operands::WriteReadImmediate, 2),
    ("jmp", Opcode::Jump, Operands::Label, 1),
    ("jz", Opcode::JumpIfZero, Operands::ReadLabel, 1),
    ("jnz", Opcode::JumpIfNotZero, Operands::ReadLabel, 1),
    ("jlt", Opcode::JumpIfLess, Operands::ReadReadLabel, 1),
    ("jle", Opcode::JumpIfLessOrEqual, Operands::ReadReadLabel, 1),
    ("jgt", Opcode::JumpIfGreater, Operands::ReadReadLabel, 1),
    (
        "jge",
        Opcode::JumpIfGreaterOrEqual,
        Operands::ReadReadLabel,
        1,
    ),
    ("score", Opcode::Score, Operands::Read, 1),
    ("halt", Opcode::Halt, Operands::Nothing, 1),
    ("material", Opcode::Material, Operands::Write, 32),
    ("gain", Opcode::Gain, Operands::Write, 1),
    ("mobility", Opcode::Mobility, Operands::Write, 200),
    ("oppmob", Opcode::OpponentMobility, Operands::Write, 200),
    ("givescheck", Opcode::GivesCheck, Operands::Write, 1),
    ("iscapture", Opcode::IsCapture, Operands::Write, 1),
    ("iscastle", Opcode::IsCastle, Operands::Write, 1),
    ("ispromo", Opcode::IsPromotion, Operands::Write, 1),
    ("promo", Opcode::Promotion, Operands::Write, 1),
    ("matein1", Opcode::MateInOne, Operands::Write, 200),
    ("kingdist", Opcode::KingDistance, Operands::Write, 2),
    ("ownkingdist", Opcode::OwnKingDistance, Operands::Write, 32),
    (
        "enemykingdist",
        Opcode::EnemyKingDistance,
        Operands::Write,
        32,
    ),
    ("exposed", Opcode::Exposed, Operands::Write, 96),
    ("lightcount", Opcode::LightCount, Operands::Write, 32),
    ("attackers", Opcode::Attackers, Operands::WriteRead, 48),
    ("defenders", Opcode::Defenders, Operands::WriteRead, 48),
    ("fromsq", Opcode::FromSquare, Operands::Write, 1),
    ("tosq", Opcode::ToSquare, Operands::Write, 1),
    ("torank", Opcode::ToRank, Operands::Write, 1),
    ("movedpiece", Opcode::MovedPiece, Operands::Write, 1),
    ("center", Opcode::Center, Operands::Write, 1),
    ("pieceat", Opcode::PieceAt, Operands::WriteRead, 2),
    ("see", Opcode::StaticExchange, Operands::Write, 128),
    ("hangs", Opcode::Hangs, Operands::Write, 128),
    ("safe", Opcode::Safe, Operands::Write, 96),
    ("movecount", Opcode::MoveCount, Operands::WriteRead, 2),
    ("ply", Opcode::Ply, Operands::Write, 1),
];

fn mnemonic(name: &str) -> Option<(Opcode, Operands, u64)> {
    MNEMONICS
        .iter()
        .find(|(written, ..)| *written == name)
        .map(|&(_, opcode, operands, cycles)| (opcode, operands, cycles))
}

#[derive(Clone, Copy)]
pub struct Instruction {
    pub opcode: Opcode,
    pub cycles: u64,
    pub destination: usize,
    pub left: usize,
    pub right: usize,
    pub immediate: i32,
    pub line: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Author {
    Agent,
    Opponent,
}

pub struct Program {
    code: Vec<Instruction>,
    data: Vec<i32>,
    search_depth: i32,
    author: Author,
}

impl Program {
    pub fn code(&self) -> &[Instruction] {
        &self.code
    }

    pub fn data(&self) -> &[i32] {
        &self.data
    }

    pub fn search_depth(&self) -> i32 {
        self.search_depth
    }

    pub fn budget(&self) -> u64 {
        match self.author {
            Author::Agent => CYCLE_BUDGET,
            Author::Opponent => u64::MAX,
        }
    }
}

pub fn assemble(source: &str, author: Author) -> Result<Program, String> {
    let mut assembler = Assembler::default();
    assembler.scan(source, author)?;

    if author == Author::Agent && assembler.instructions.len() > CODE_LIMIT {
        return Err(format!(
            "the code section is {} instructions, the limit is {CODE_LIMIT} (the first instruction over it is on line {})",
            assembler.instructions.len(),
            assembler.instructions[CODE_LIMIT].line
        ));
    }

    assembler.fill_data(source)?;

    let mut code = Vec::with_capacity(assembler.instructions.len());
    for source_line in &assembler.instructions {
        code.push(assembler.encode(source_line)?);
    }
    if code.is_empty() {
        return Err(
            "the code section is empty, a program needs at least a SCORE and a HALT".to_string(),
        );
    }

    Ok(Program {
        code,
        data: assembler.data,
        search_depth: assembler.search_depth,
        author,
    })
}

struct SourceLine {
    line: usize,
    tokens: Vec<String>,
}

#[derive(Default)]
struct Assembler {
    labels: Vec<(String, i32)>,
    instructions: Vec<SourceLine>,
    data: Vec<i32>,
    search_depth: i32,
}

impl Assembler {
    fn scan(&mut self, source: &str, author: Author) -> Result<(), String> {
        self.search_depth = NO_SEARCH;
        let mut in_data = false;

        for (index, text) in source.lines().enumerate() {
            let line = index + 1;
            let mut tokens = tokenize(text);
            if tokens.is_empty() {
                continue;
            }

            if let Some(name) = tokens[0].strip_suffix(LABEL_END).map(str::to_string) {
                if name.is_empty() {
                    return Err(format!("line {line}: a label needs a name before the ':'"));
                }
                let address = if in_data {
                    DATA_BASE + self.data.len() as i32
                } else {
                    self.instructions.len() as i32
                };
                self.define(&name, address, line)?;
                tokens.remove(0);
                if tokens.is_empty() {
                    continue;
                }
            }

            if tokens[0].eq_ignore_ascii_case(SEARCH_DIRECTIVE) {
                self.search(&tokens, author, in_data, line)?;
                continue;
            }

            if tokens[0].eq_ignore_ascii_case(DATA_DIRECTIVE) {
                if tokens.len() > 1 {
                    return Err(format!("line {line}: '{DATA_DIRECTIVE}' takes no operands"));
                }
                in_data = true;
                continue;
            }

            if !in_data {
                if tokens[0].starts_with('.') {
                    return Err(format!(
                        "line {line}: '{}' is only valid after a '{DATA_DIRECTIVE}' line",
                        tokens[0]
                    ));
                }
                self.instructions.push(SourceLine { line, tokens });
                continue;
            }

            self.reserve_data(&tokens, line)?;

            if author == Author::Agent && self.data.len() > DATA_LIMIT {
                return Err(format!(
                    "line {line}: the data section is {} words, the limit is {DATA_LIMIT}",
                    self.data.len()
                ));
            }
        }

        Ok(())
    }

    fn search(
        &mut self,
        tokens: &[String],
        author: Author,
        in_data: bool,
        line: usize,
    ) -> Result<(), String> {
        if in_data {
            return Err(format!(
                "line {line}: '{SEARCH_DIRECTIVE}' must come before the '{DATA_DIRECTIVE}' section"
            ));
        }
        if author == Author::Agent {
            return Err(format!(
                "line {line}: '{SEARCH_DIRECTIVE}' is not available to a submission, which scores the moves of one position without looking ahead"
            ));
        }
        if tokens.len() != 2 {
            return Err(format!(
                "line {line}: '{SEARCH_DIRECTIVE}' takes one depth, got {}",
                tokens.len() - 1
            ));
        }

        let depth = parse_number(&tokens[1])
            .ok_or_else(|| format!("line {line}: '{}' is not a depth", tokens[1]))?;
        if depth < NO_SEARCH {
            return Err(format!(
                "line {line}: '{SEARCH_DIRECTIVE} {depth}' must be at least {NO_SEARCH}"
            ));
        }
        self.search_depth = depth;

        Ok(())
    }

    fn reserve_data(&mut self, tokens: &[String], line: usize) -> Result<(), String> {
        let operands = &tokens[1..];

        match tokens[0].to_ascii_lowercase().as_str() {
            WORD_DIRECTIVE => {
                if operands.is_empty() {
                    return Err(format!(
                        "line {line}: '{WORD_DIRECTIVE}' needs at least one value"
                    ));
                }
                self.data.resize(self.data.len() + operands.len(), 0);
            }
            ZERO_DIRECTIVE => {
                if operands.len() != 1 {
                    return Err(format!(
                        "line {line}: '{ZERO_DIRECTIVE}' takes one count, got {}",
                        operands.len()
                    ));
                }
                let count = parse_number(&operands[0])
                    .ok_or_else(|| format!("line {line}: '{}' is not a number", operands[0]))?;
                if count < 0 {
                    return Err(format!(
                        "line {line}: '{ZERO_DIRECTIVE} {count}' is negative"
                    ));
                }
                self.data.resize(self.data.len() + count as usize, 0);
            }
            _ => return Err(format!("line {line}: unknown directive '{}'", tokens[0])),
        }

        Ok(())
    }

    fn fill_data(&mut self, source: &str) -> Result<(), String> {
        let mut in_data = false;
        let mut address = 0usize;

        for (index, text) in source.lines().enumerate() {
            let line = index + 1;
            let mut tokens = tokenize(text);
            if tokens.is_empty() {
                continue;
            }
            if tokens[0].ends_with(LABEL_END) {
                tokens.remove(0);
                if tokens.is_empty() {
                    continue;
                }
            }
            if tokens[0].eq_ignore_ascii_case(DATA_DIRECTIVE) {
                in_data = true;
                continue;
            }
            if !in_data {
                continue;
            }

            match tokens[0].to_ascii_lowercase().as_str() {
                WORD_DIRECTIVE => {
                    for token in &tokens[1..] {
                        self.data[address] = self.immediate(token, line)?;
                        address += 1;
                    }
                }
                ZERO_DIRECTIVE => address += parse_number(&tokens[1]).unwrap_or(0) as usize,
                _ => {}
            }
        }

        Ok(())
    }

    fn define(&mut self, name: &str, address: i32, line: usize) -> Result<(), String> {
        if mnemonic(&name.to_ascii_lowercase()).is_some() {
            return Err(format!(
                "line {line}: '{name}' is an instruction name and cannot be a label"
            ));
        }
        if self.lookup(name).is_some() {
            return Err(format!("line {line}: label '{name}' is defined twice"));
        }
        self.labels.push((name.to_string(), address));

        Ok(())
    }

    fn lookup(&self, name: &str) -> Option<i32> {
        self.labels
            .iter()
            .find(|(known, _)| known == name)
            .map(|&(_, address)| address)
    }

    fn immediate(&self, token: &str, line: usize) -> Result<i32, String> {
        parse_number(token)
            .or_else(|| self.lookup(token))
            .ok_or_else(|| format!("line {line}: '{token}' is neither a number nor a label"))
    }

    fn label(&self, token: &str, line: usize) -> Result<i32, String> {
        self.lookup(token)
            .ok_or_else(|| format!("line {line}: jump target '{token}' is not a defined label"))
    }

    fn encode(&self, source_line: &SourceLine) -> Result<Instruction, String> {
        let SourceLine { line, tokens } = source_line;
        let line = *line;

        let Some((opcode, operands, cycles)) = mnemonic(&tokens[0].to_ascii_lowercase()) else {
            return Err(format!("line {line}: unknown instruction '{}'", tokens[0]));
        };

        let written = &tokens[1..];
        if written.len() != operands.count() {
            return Err(format!(
                "line {line}: '{}' takes {} operand{}, got {}",
                tokens[0],
                operands.count(),
                if operands.count() == 1 { "" } else { "s" },
                written.len()
            ));
        }

        let mut instruction = Instruction {
            opcode,
            cycles,
            destination: 0,
            left: 0,
            right: 0,
            immediate: 0,
            line,
        };

        match operands {
            Operands::WriteReadRead => {
                instruction.destination = parse_register(&written[0], line)?;
                instruction.left = parse_register(&written[1], line)?;
                instruction.right = parse_register(&written[2], line)?;
            }
            Operands::WriteRead => {
                instruction.destination = parse_register(&written[0], line)?;
                instruction.left = parse_register(&written[1], line)?;
            }
            Operands::WriteImmediate => {
                instruction.destination = parse_register(&written[0], line)?;
                instruction.immediate = self.immediate(&written[1], line)?;
            }
            Operands::WriteReadImmediate => {
                instruction.destination = parse_register(&written[0], line)?;
                instruction.left = parse_register(&written[1], line)?;
                instruction.immediate = self.immediate(&written[2], line)?;
            }
            Operands::Label => instruction.immediate = self.label(&written[0], line)?,
            Operands::ReadLabel => {
                instruction.left = parse_register(&written[0], line)?;
                instruction.immediate = self.label(&written[1], line)?;
            }
            Operands::ReadReadLabel => {
                instruction.left = parse_register(&written[0], line)?;
                instruction.right = parse_register(&written[1], line)?;
                instruction.immediate = self.label(&written[2], line)?;
            }
            Operands::Read | Operands::Write => {
                let register = parse_register(&written[0], line)?;
                if operands == Operands::Read {
                    instruction.left = register;
                } else {
                    instruction.destination = register;
                }
            }
            Operands::Nothing => {}
        }

        Ok(instruction)
    }
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(COMMENT)
        .next()
        .unwrap_or_default()
        .replace(',', " ")
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

fn parse_register(token: &str, line: usize) -> Result<usize, String> {
    let digits = REGISTER_PREFIX
        .iter()
        .find_map(|prefix| token.strip_prefix(*prefix));

    match digits.and_then(|digits| digits.parse::<usize>().ok()) {
        Some(register) if register < REGISTERS => Ok(register),
        _ => Err(format!(
            "line {line}: '{token}' is not a register, the registers are r0 through r{}",
            REGISTERS - 1
        )),
    }
}

fn parse_number(token: &str) -> Option<i32> {
    let (sign, digits) = match token.strip_prefix('-') {
        Some(rest) => (-1i64, rest),
        None => (1i64, token),
    };

    let magnitude = match HEX_PREFIX
        .iter()
        .find_map(|prefix| digits.strip_prefix(*prefix))
    {
        Some(hex) => i64::from_str_radix(hex, 16).ok()?,
        None => digits.parse::<i64>().ok()?,
    };

    Some((sign * magnitude) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(source: &str) -> Result<Program, String> {
        assemble(source, Author::Agent)
    }

    fn rejection(source: &str) -> String {
        match agent(source) {
            Ok(_) => panic!("the assembler accepted an invalid program"),
            Err(error) => error,
        }
    }

    #[test]
    fn assemble_returns_the_instructions_and_the_data() {
        let program = agent(
            "
                ldi r1, values
                ld r2, r1, 1
                score r2
                halt
            .data
            values: .word 100, 320, 330
                    .zero 2
            ",
        )
        .expect("the program assembles");

        assert_eq!(program.code().len(), 4);
        assert_eq!(program.data(), [100, 320, 330, 0, 0]);
        assert_eq!(
            program.code()[0].immediate,
            0,
            "a data label is the address of the word after it"
        );
    }

    #[test]
    fn a_code_label_is_the_address_of_the_next_instruction() {
        let program = agent(
            "
                jmp done
                halt
            done:
                score r0
                halt
            ",
        )
        .expect("the program assembles");

        assert_eq!(program.code()[0].immediate, 2);
    }

    #[test]
    fn only_an_opponent_may_use_search() {
        let search = "
            .search 3
                material r0
                score r0
                halt
            ";

        assert_eq!(
            assemble(search, Author::Opponent)
                .expect("an opponent may search")
                .search_depth(),
            3
        );

        let rejected = rejection(search);
        assert!(rejected.contains("line 2"), "{rejected}");
        assert!(rejected.contains(".search"), "{rejected}");
    }

    #[test]
    fn a_submission_has_code_and_data_limits() {
        let long = "    ldi r0, 1\n".repeat(CODE_LIMIT + 1) + "    halt\n";
        let rejected = rejection(&long);
        assert!(
            rejected.contains(&format!("{}", CODE_LIMIT + 1)),
            "{rejected}"
        );
        assert!(
            rejected.contains(&format!("line {}", CODE_LIMIT + 1)),
            "{rejected}"
        );

        let wide = format!("    halt\n.data\n    .zero {}\n", DATA_LIMIT + 1);
        let rejected = rejection(&wide);
        assert!(rejected.contains("line 3"), "{rejected}");
        assert!(
            rejected.contains(&format!("{}", DATA_LIMIT + 1)),
            "{rejected}"
        );
    }

    #[test]
    fn an_opponent_has_no_code_limit() {
        let long = "    ldi r0, 1\n".repeat(CODE_LIMIT + 1) + "    halt\n";

        assert!(assemble(&long, Author::Opponent).is_ok());
    }

    #[test]
    fn an_error_names_the_line_it_is_on() {
        for (source, expected) in [
            (
                "    halt\n    frobnicate r0\n",
                "line 2: unknown instruction",
            ),
            (
                "    ldi r16, 1\n    halt\n",
                "line 1: 'r16' is not a register",
            ),
            (
                "    add r0, r1\n    halt\n",
                "line 1: 'add' takes 3 operands",
            ),
            (
                "    jmp nowhere\n    halt\n",
                "line 1: jump target 'nowhere'",
            ),
            (
                "    ldi r0, nothing\n    halt\n",
                "line 1: 'nothing' is neither",
            ),
            (
                "here:\nhere:\n    halt\n",
                "line 2: label 'here' is defined twice",
            ),
            (
                "score:\n    halt\n",
                "line 1: 'score' is an instruction name",
            ),
            ("; nothing at all\n", "the code section is empty"),
        ] {
            let rejected = rejection(source);
            assert!(
                rejected.starts_with(expected),
                "expected {expected:?}, got {rejected:?}"
            );
        }
    }

    #[test]
    fn commas_and_case_and_comments_are_read_as_whitespace() {
        let program = agent("LDI r0,1 ; a comment\n    HALT\n")
            .expect("mnemonics are case insensitive and commas optional");

        assert_eq!(program.code().len(), 2);
        assert_eq!(program.code()[0].opcode, Opcode::LoadImmediate);
        assert_eq!(program.code()[0].immediate, 1);
    }

    #[test]
    fn immediates_are_written_in_decimal_or_hex() {
        assert_eq!(parse_number("42"), Some(42));
        assert_eq!(parse_number("-42"), Some(-42));
        assert_eq!(parse_number("0x2A"), Some(42));
        assert_eq!(parse_number("-0x2a"), Some(-42));
        assert_eq!(parse_number("4 2"), None);
        assert_eq!(parse_number("what"), None);
    }

    #[test]
    fn every_opponent_assembles() {
        for (name, source) in crate::chess_vm::field::OPPONENTS {
            assert!(
                assemble(source, Author::Opponent).is_ok(),
                "the opponent {name} does not assemble"
            );
        }
    }

    #[test]
    fn the_starter_and_the_reference_are_within_the_limits_of_a_submission() {
        for source in [include_str!("task/bot.cvm"), include_str!("reference.cvm")] {
            assert!(agent(source).is_ok());
        }
    }
}
