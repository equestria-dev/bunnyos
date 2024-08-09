use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

#[derive(Debug)]
pub struct Command {
    pub command: String,
    pub args: BTreeMap<String, CommandArgument>,
    pub names: Vec<String>
}

#[derive(Debug)]
pub enum CommandArgument {
    Anonymous,
    Value(String)
}

#[derive(Debug)]
pub enum CommandError {
    ExecutablePathNotFound,
    MismatchedQuotes
}

impl From<&Command> for Vec<u8> {
    fn from(value: &Command) -> Self {
        let mut bytes = vec![];

        bytes.append(&mut (value.command.len() as u64).to_le_bytes().to_vec());
        bytes.append(&mut value.command.as_bytes().to_vec());
        bytes.append(&mut (value.args.len() as u64).to_le_bytes().to_vec());

        for (name, value) in &value.args {
            match value {
                CommandArgument::Anonymous => {
                    bytes.push(0);
                    bytes.append(&mut (name.len() as u64).to_le_bytes().to_vec());
                    bytes.append(&mut name.as_bytes().to_vec());
                }
                CommandArgument::Value(value) => {
                    bytes.push(1);
                    bytes.append(&mut (name.len() as u64).to_le_bytes().to_vec());
                    bytes.append(&mut name.as_bytes().to_vec());
                    bytes.append(&mut (value.len() as u64).to_le_bytes().to_vec());
                    bytes.append(&mut value.as_bytes().to_vec());
                }
            }
        }

        bytes.append(&mut (value.names.len() as u64).to_le_bytes().to_vec());

        for name in &value.names {
            bytes.append(&mut (name.len() as u64).to_le_bytes().to_vec());
            bytes.append(&mut name.as_bytes().to_vec());
        }

        bytes
    }
}

impl Command {
    #[allow(clippy::manual_strip)] pub fn build(input: &str) -> Result<Self, CommandError> {
        let mut in_double_quotes = false;
        let mut in_single_quotes = false;
        let mut escaping = false;

        let input = input.trim();
        let input_split = input.split(|char| match char {
            '"' if !in_single_quotes && !escaping => {
                in_double_quotes = !in_double_quotes;
                false
            }
            '\'' if !in_double_quotes && !escaping => {
                in_single_quotes = !in_single_quotes;
                false
            }
            '\\' if !escaping => {
                escaping = true;
                false
            },
            ' ' => !in_double_quotes && !in_single_quotes,
            _ => {
                escaping = false;
                false
            },
        });

        let mut parsed = input_split.map(|split| {
            match split.chars().next().unwrap_or(' ') {
                '\'' | '"' => {
                    let end = split.len() - 1;
                    split[1..end].replace("\\n", "\n").replace("\\t", "\t").replace("\\r", "\r").replace('\\', "")
                }
                _ => split.replace("\\n", "\n").replace("\\t", "\t").replace("\\r", "\r").replace('\\', "")
            }
        });
        let command = parsed.next().ok_or(CommandError::ExecutablePathNotFound)?;
        let args: Vec<String> = parsed.collect();

        if in_double_quotes || in_single_quotes {
            return Err(CommandError::MismatchedQuotes);
        }

        let mut command_args: BTreeMap<String, CommandArgument> = BTreeMap::new();
        let mut names: Vec<String> = Vec::new();

        for arg in args.iter() {
            if arg.starts_with("--") {
                if arg.contains('=') {
                    let parts: Vec<&str> = arg.split('=').collect();
                    let mut value: &str = parts[1];

                    if value.starts_with('"') || value.starts_with('\'') {
                        let len = value.len() - 1;
                        value = &value[1..len];
                    }

                    command_args.insert(parts[0][2..].parse().unwrap(), CommandArgument::Value(String::from(value)));
                } else {
                    command_args.insert(arg[2..].parse().unwrap(), CommandArgument::Anonymous);
                }
            } else if arg.starts_with('-') {
                let chars: String = arg[1..].parse().unwrap();

                for i in chars.chars() {
                    command_args.insert(i.to_string(), CommandArgument::Anonymous);
                }
            } else {
                names.push(arg.parse().unwrap());
            }
        }

        Ok(Command {
            command,
            args: command_args,
            names
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.into()
    }
}
