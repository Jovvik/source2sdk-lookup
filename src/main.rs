#![feature(if_let_guard)]
use anyhow::Result;
use colored::*;
use dotenv_codegen::dotenv;
use serde::Deserialize;
use std::{
    collections::HashMap,
    io::{stdin, stdout, Write},
};

#[derive(Debug, Deserialize)]
struct Sdk {
    #[serde(flatten)]
    type_scopes: HashMap<String, TypeScope>,
}

#[derive(Debug, Deserialize)]
struct TypeScope {
    #[serde(flatten)]
    classes: HashMap<String, Class>,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct Class {
    #[serde(flatten)]
    fields: HashMap<String, Field>,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct Field {
    offset: usize,
    type_: String,
}

#[derive(Debug)]
struct FieldEntry {
    name: String,
    type_: String,
    class_name: String,
    type_scope_name: String,
}

fn make_offset_to_fields(sdk: &Sdk) -> HashMap<usize, Vec<FieldEntry>> {
    let mut offset_to_fields = HashMap::new();
    for (type_scope_name, type_scope) in &sdk.type_scopes {
        for (class_name, class) in &type_scope.classes {
            for (field_name, field) in &class.fields {
                offset_to_fields
                    .entry(field.offset)
                    .or_insert_with(Vec::new)
                    .push(FieldEntry {
                        name: field_name.clone(),
                        type_: field.type_.clone(),
                        class_name: class_name.clone(),
                        type_scope_name: type_scope_name.clone(),
                    });
            }
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
                        println!(
                            "{} {}{}{} ({})",
                            field.type_.purple(),
                            field.class_name.yellow(),
                            "::".dimmed(),
                            field.name,
                            field.type_scope_name.dimmed(),
                        );
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
    let path = dotenv!("SCHEMA_JSON");
    let sdk: Sdk = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let offset_to_fields = make_offset_to_fields(&sdk);
    run_interactive_loop(&offset_to_fields)?;
    Ok(())
}
