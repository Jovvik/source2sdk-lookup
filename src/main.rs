#![feature(if_let_guard)]
use anyhow::Result;
use colored::*;
use dotenv_codegen::dotenv;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::File,
    io::{stdin, stdout, BufReader, Write},
    path::Path,
};

#[derive(Debug, Deserialize)]
struct Sdk {
    #[serde(flatten)]
    type_scopes: HashMap<String, TypeScope>,
}
impl Sdk {
    fn from_path(path: &Path) -> Result<Self> {
        let mut type_scopes = HashMap::new();
        for type_scope_path in path.read_dir()? {
            let type_scope_path = type_scope_path?.path();
            println!("loading {}", type_scope_path.display());
            let file = File::open(type_scope_path)?;
            let reader = BufReader::new(file);
            let sdk: Sdk = serde_json::from_reader(reader)?;
            type_scopes.extend(sdk.type_scopes);
        }
        Ok(Self { type_scopes })
    }
}

#[derive(Debug, Deserialize)]
struct TypeScope {
    classes: HashMap<String, Class>,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct Class {
    fields: HashMap<String, usize>,
    metadata: Vec<ClassMetadata>,
}
impl Class {
    fn get_field_type(&self, name: &str) -> Option<String> {
        self.metadata
            .iter()
            .filter_map(|metadata| match metadata {
                ClassMetadata::NetworkVarNames { name: n, type_name } if n == name => {
                    Some(type_name.clone())
                }
                _ => None,
            })
            .next()
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
enum ClassMetadata {
    Unknown { name: String },
    NetworkChangeCallback { name: String },
    NetworkVarNames { name: String, type_name: String },
}

#[derive(Debug)]
struct FieldEntry {
    name: String,
    type_: Option<String>,
    class_name: String,
    type_scope_name: String,
}

fn make_offset_to_fields(sdk: &Sdk) -> HashMap<usize, Vec<FieldEntry>> {
    let mut offset_to_fields = HashMap::new();
    for (type_scope_name, type_scope) in &sdk.type_scopes {
        for (class_name, class) in &type_scope.classes {
            for (field_name, offset) in &class.fields {
                offset_to_fields
                    .entry(*offset)
                    .or_insert_with(Vec::new)
                    .push(FieldEntry {
                        name: field_name.clone(),
                        type_: class.get_field_type(field_name),
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
                            field
                                .type_
                                .as_ref()
                                .map(|type_| type_.purple())
                                .unwrap_or("Unknown type".red()),
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
    let path = Path::new(dotenv!("SCHEMA_DIR"));
    let sdk = Sdk::from_path(path)?;
    let offset_to_fields = make_offset_to_fields(&sdk);
    run_interactive_loop(&offset_to_fields)?;
    Ok(())
}
