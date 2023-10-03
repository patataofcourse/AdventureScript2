use crate::core::{
    error::{ASCmdError, CommandErrors},
    ASType, ASVariable, GameInfo,
};
use std::collections::HashMap;

mod definitions;

#[derive(Clone)]
pub struct Command {
    pub name: String,
    func: CommandFn,
    args: Vec<CommandArg>,
    pub deprecated: bool,
}

//TODO: Fn rather than fn
pub type CommandFn = fn(&mut GameInfo, Vec<ASVariable>) -> anyhow::Result<()>;

#[derive(Clone, Debug)]
pub struct CommandArg {
    pub name: String,
    pub type_: ASType,
    pub required: bool,
}

//TODO: assert the CmdSet is properly constructed (no duplicates)
#[derive(Clone)]
pub struct CmdSet {
    pub commands: Vec<Command>,
    pub aliases: HashMap<String, String>, //TODO: include aliases in command definitions
    pub modules: HashMap<String, CmdSet>, //TODO: make it work
}

impl CmdSet {
    pub fn get(&self, name: &str) -> Option<&Command> {
        let (module, name) = name.split_once('.').unwrap_or(("", name));
        let module = if !module.is_empty() {
            self.modules.get(module)?
        } else {
            self
        };
        if let Some(command) = module.commands.iter().find(|cmd| cmd.name == name) {
            return Some(command);
        }
        module
            .aliases
            .get(name)
            .map(|c| module.get(c))
            .unwrap_or(None)
    }
    pub fn extend(&mut self, other: Self) {
        self.commands.extend(other.commands);
        self.aliases.extend(other.aliases);
    }
    pub fn from(commands: Vec<Command>, aliases: HashMap<String, String>) -> Self {
        Self {
            commands,
            aliases,
            modules: HashMap::new(),
        }
    }
    pub fn new() -> Self {
        Self {
            commands: vec![],
            aliases: HashMap::new(),
            modules: HashMap::new(),
        }
    }
    pub(crate) fn add_module(&mut self, commands: CmdSet, name: &str) {
        self.modules.insert(name.to_string(), commands);
    }
}

impl Default for CmdSet {
    fn default() -> Self {
        Self::new()
    }
}

impl Command {
    pub fn new(
        name: String,
        func: CommandFn,
        args: Vec<CommandArg>,
        deprecated: bool,
    ) -> anyhow::Result<Self> {
        let mut optionals = false;
        for arg in &args {
            if arg.type_ == ASType::None {
                Err(ASOtherError::DevErr(format!(
                    "Command '{}' takes a None type parameter",
                    name
                )))?
            }

            if optionals && arg.required {
                Err(ASOtherError::DevErr(format!(
                    "Command '{}' has a required argument after an optional one",
                    name
                )))?
            } else if !arg.required {
                optionals = true
            }
        }

        Ok(Self {
            name,
            func,
            args,
            deprecated,
        })
    }
    pub fn run(
        &self,
        info: &mut GameInfo,
        mut args: Vec<ASVariable>,
        kwargs: HashMap<String, ASVariable>,
    ) -> anyhow::Result<()> {
        // Check that there's not too many arguments
        if args.len() > self.args.len() {
            Err(ASCmdError {
                command: self.name.to_string(),
                details: CommandErrors::TooManyArguments {
                    max_args: self.args.len(),
                    given_args: args.len(),
                },
            })?
        }

        // Expand args to the size of all the arguments
        for _ in 0..(self.args.len() - args.len()) {
            args.push(ASVariable::None);
        }

        // Pass kwargs to args
        for (k, v) in &kwargs {
            let Some(pos) = self.args.iter().position(|c| c.name == *k) else {
                Err(ASCmdError {
                    command: self.name.to_string(),
                    details: CommandErrors::UndefinedArgument {
                        argument_name: k.clone(),
                        argument_type: v.get_type(),
                    },
                })?
            };

            args[pos] = v.clone();
        }

        // Check argument types + that no required arg is None
        let mut check_required = true;
        for c in 0..args.iter().len() {
            let arg_def = &self.args[c];
            if !arg_def.required {
                check_required = false;
            }

            let arg_type = args[c].get_type();

            if args[c] == ASVariable::None && arg_def.required && check_required {
                Err(ASCmdError {
                    command: self.name.clone(),
                    details: CommandErrors::MissingRequiredArgument {
                        argument_name: arg_def.name.clone(),
                        argument_type: arg_def.type_.clone(),
                    },
                })?
            } else if !(arg_def.type_ == ASType::Any && arg_type != ASType::VarRef)
                && arg_def.type_ != arg_type
            {
                if arg_type == ASType::VarRef {
                    let ASVariable::VarRef(var) = &args[c] else {
                        unreachable!()
                    };
                    args[c] = info.get_var(var)?.clone()
                } else if arg_type == ASType::None && arg_def.type_ == ASType::Label {
                    args[c] = ASVariable::Label(None.into())
                } else if !(arg_type == ASType::None && !arg_def.required) {
                    Err(ASCmdError {
                        command: String::from(&self.name),
                        details: CommandErrors::ArgumentTypeError {
                            argument_name: arg_def.name.clone(),
                            required_type: arg_def.type_.clone(),
                            given_type: arg_type,
                        },
                    })?
                }
            }
        }

        (self.func)(info, args)
    }
}

pub use definitions::main_commands;

use super::error::ASOtherError;
