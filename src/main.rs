#![feature(if_let_guard)]
use anyhow::{bail, Context, Result};
use dotenv_codegen::dotenv;
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    collections::HashMap,
    fs::File,
    io::{stdin, stdout, BufRead, BufReader, Write},
};
use colored::*;

#[derive(Debug, PartialEq, Eq)]
struct Field {
    name: String,
    type_: String,
    offset: usize,
}

#[derive(Debug, PartialEq, Eq)]
struct Class {
    name: String,
    fields: Vec<Field>,
}

#[derive(Debug)]
struct FieldEntry {
    name: String,
    type_: String,
    class_name: String,
}

#[derive(Debug)]
struct Sdk {
    classes: Vec<Class>,
}

const SKIP_LINES: [&str; 4] = ["public:", "private:", "{", "}"];

#[derive(PartialEq, Eq)]
enum ClassState {
    TopLevel,
    Bitfields,
}

#[derive(PartialEq, Eq)]
enum State {
    TopLevel,
    Class_(ClassState, Class),
    Enum,
}

lazy_static! {
    static ref FIELD_REGEX: Regex = Regex::new(r"(.*) (\w+(\[\d+\])*); // 0x([0-9a-fA-F]+)$").unwrap();
    static ref PADDING_REGEX: Regex =
        Regex::new(r"\[\[maybe_unused\]\] uint8_t __pad[0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F]\[0x[0-9a-fA-F]+\]; // 0x[0-9a-fA-F]+$").unwrap();
    static ref BITFIELD_REGEX: Regex = Regex::new(r"uint(8|16)_t \w+: \d+;").unwrap();
    static ref STATIC_FIELD_REGEX: Regex = Regex::new(r"static .* &Get_\w+\(\)\{return .*;\};$").unwrap();
    static ref ENUM_VALUE_REGEX: Regex = Regex::new(r"\w+ = 0x[0-9a-fA-F]+,").unwrap();
}

fn process_line(
    line: std::io::Result<String>,
    mut state: State,
    classes: &mut Vec<Class>,
) -> Result<State> {
    let line = line?.trim().to_string();
    match line.as_str() {
        "" => Ok(state),
        l if l.starts_with("//") => Ok(state),
        l if l.starts_with("#pragma") || l.starts_with("#include") => Ok(state),
        l if SKIP_LINES.contains(&l) => Ok(state),

        l if l.starts_with("struct") && l.ends_with(';') => Ok(state), // Forward declaration
        l if l.starts_with("};") => match state {
            State::Class_(ClassState::TopLevel, cur_class) => {
                classes.push(cur_class);
                Ok(State::TopLevel)
            }
            State::Class_(ClassState::Bitfields, cur_class) => {
                Ok(State::Class_(ClassState::TopLevel, cur_class))
            }
            State::Enum => Ok(State::TopLevel),
            State::TopLevel => bail!("Unexpected end of class or enum"),
        },

        l if l.starts_with("enum class") => match state {
            State::TopLevel => Ok(State::Enum),
            State::Class_(_, cur_class) => bail!("Unexpected enum inside class {}", cur_class.name),
            State::Enum => bail!("Unexpected nested enum"),
        },
        "struct" => match state {
            State::Class_(ClassState::TopLevel, cur_class) => {
                Ok(State::Class_(ClassState::Bitfields, cur_class))
            }
            _ => bail!("Unexpected bitfield struct outside of class"),
        },
        l if l.starts_with("class ") || l.starts_with("struct ") => {
            let name = l.split_whitespace().nth(1).unwrap().to_string();
            if let State::Class_(_, cur_class) = state {
                bail!("Unexpected nested class {} inside {}", name, cur_class.name)
            }
            Ok(State::Class_(
                ClassState::TopLevel,
                Class { name, fields: vec![] },
            ))
        }

        _ if PADDING_REGEX.is_match(&line) => match state {
            State::Class_(ClassState::TopLevel, _) => Ok(state),
            _ => bail!("Unexpected padding outside of class"),
        },
        _ if STATIC_FIELD_REGEX.is_match(&line) => match state {
            State::Class_(ClassState::TopLevel, _) => Ok(state),
            _ => bail!("Unexpected static field outside of class"),
        },
        _ if ENUM_VALUE_REGEX.is_match(&line) => match state {
            State::Enum => Ok(state),
            _ => bail!("Unexpected enum value outside of enum"),
        },
        _ if BITFIELD_REGEX.is_match(&line) => match state {
            State::Class_(ClassState::Bitfields, _) => Ok(state),
            State::Class_(ClassState::TopLevel, _) => {
                bail!("Unexpected bitfield outside of bitfield struct")
            }
            State::TopLevel => bail!("Unexpected bitfield outside of class"),
            State::Enum => bail!("Unexpected bitfield inside enum"),
        },
        _ if let Some(captures) = FIELD_REGEX.captures(&line) => {
            let type_ = captures.get(1).unwrap().as_str().to_string();
            let name = captures.get(2).unwrap().as_str().to_string();
            let offset = usize::from_str_radix(captures.get(4).unwrap().as_str(), 16)?;
            if let State::Class_(ClassState::TopLevel, cur_class) = &mut state {
                cur_class.fields.push(Field {
                    name,
                    type_,
                    offset,
                });
            } else {
                bail!(
                    "Unexpected field {} outside of class or inside a bitfield struct",
                    name
                )
            }
            Ok(state)
        },

        _ => bail!("Unrecognized line"),
    }
}

fn parse_sdk(filename: &str) -> Result<Sdk> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let mut classes = vec![];
    let mut state: State = State::TopLevel;
    for (idx, line) in reader.lines().enumerate() {
        state = process_line(line, state, &mut classes)
            .with_context(|| format!("Error parsing line {}", idx + 1))?;
    }
    match state {
        State::TopLevel => {}
        State::Class_(_, cur_class) => {
            bail!("Unexpected end of file inside class {}", cur_class.name)
        }
        State::Enum => bail!("Unexpected end of file inside enum"),
    }
    Ok(Sdk { classes })
}

fn make_offset_to_fields(sdk: &Sdk) -> HashMap<usize, Vec<FieldEntry>> {
    let mut offset_to_fields = HashMap::new();
    for class in &sdk.classes {
        for field in &class.fields {
            offset_to_fields
                .entry(field.offset)
                .or_insert_with(Vec::new)
                .push(FieldEntry {
                    name: field.name.clone(),
                    type_: field.type_.clone(),
                    class_name: class.name.clone(),
                });
        }
    }
    offset_to_fields
}

fn run_interactive_loop(offset_to_fields: &HashMap<usize, Vec<FieldEntry>>) -> Result<()> {
    let mut input = String::new();
    loop {
        print!("enter offset {}: ", "(hex)".dimmed());
        stdout().flush()?;
        input.clear();
        stdin().read_line(&mut input)?;
        input = input.trim().to_string();
        if input.is_empty() || input == "exit" {
            break;
        }
        if input.starts_with("0x") {
            input = input[2..].to_string();
        }
        match usize::from_str_radix(&input, 16) {
            Ok(offset) => {
                if let Some(fields) = offset_to_fields.get(&offset) {
                    for field in fields {
                        println!("{} {}{}{}", field.type_.purple(), field.class_name.yellow(), "::".dimmed(), field.name);
                    }
                } else {
                    println!("no field at offset 0x{:x}", offset);
                }
            }
            Err(_) => {
                println!("invalid offset");
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let client_hpp = dotenv!("CLIENT_HPP");
    let sdk = parse_sdk(client_hpp)?;
    let offset_to_fields = make_offset_to_fields(&sdk);
    run_interactive_loop(&offset_to_fields)?;
    Ok(())
}