use crate::debugger_command::DebuggerCommand;
use crate::dwarf_data::{DwarfData, Error as DwarfError};
use crate::inferior::Inferior;
use crate::inferior::Status;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::collections::HashMap;
use gimli::RawLocListEntry::OffsetPair;

#[derive(Clone)]
struct Breakpoint {
    addr: usize,
    orig_byte: u8,
}

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<()>,
    inferior: Option<Inferior>,
    debug_data: DwarfData,
    break_points: Vec<usize>,
    brk_point_map: HashMap<usize, u8>
}

impl Debugger {
    /// Initializes the debugger.
    pub fn new(target: &str) -> Debugger {
        let history_path = format!("{}/.deet_history", std::env::var("HOME").unwrap());
        let mut readline = Editor::<()>::new();
        // Attempt to load history from ~/.deet_history if it exists
        let _ = readline.load_history(&history_path);

        let debug_data = Debugger::load_dwarf_data(target);
        debug_data.print();

        Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
            debug_data,
            break_points: Vec::new(),
            brk_point_map: HashMap::new()
        }
    }

    fn load_dwarf_data(target: &str) -> DwarfData {
        let debug_data = match DwarfData::from_file(target) {
            Ok(val) => val,
            Err(DwarfError::ErrorOpeningFile) => {
                println!("Could not open file {}", target);
                std::process::exit(1);
            }
            Err(DwarfError::DwarfFormatError(err)) => {
                println!("Could not debugging symbols from {}: {:?}", target, err);
                std::process::exit(1);
            }
        };
        return debug_data;
    }

    pub fn run(&mut self) {
        loop {
            match self.get_next_command() {
                DebuggerCommand::Run(args) => {
                    // kill the exist inferior
                    if self.inferior.is_some() {
                        self.inferior.as_mut().unwrap().kill();
                    }

                    if let Some(inferior) =
                    Inferior::new(&self.target, &args, &self.break_points, &mut self.brk_point_map) {
                        // Create the inferior
                        self.inferior = Some(inferior);
                        // TODO (milestone 1): make the inferior run
                        // You may use self.inferior.as_mut().unwrap() to get a mutable reference
                        // to the Inferior object
                        let status = self.inferior.as_mut().unwrap().continues(&self.brk_point_map).unwrap();
                        self.print_condition(status)
                    } else {
                        println!("Error starting subprocess");
                    }
                }
                DebuggerCommand::Continue => {
                    if self.inferior.is_none() || self.inferior.as_ref().unwrap().is_exited() {
                        println!("Error continue without running");
                        continue;
                    }
                    let status = self.inferior.as_mut().unwrap().continues(&self.brk_point_map).unwrap();
                    self.print_condition(status);
                }
                DebuggerCommand::Backtrace => {
                    if self.inferior.is_none() || self.inferior.as_ref().unwrap().is_exited() {
                        println!("Error backtrace without running");
                        continue;
                    }
                    self.inferior.as_ref().unwrap().print_backtrace(&self.debug_data);
                }
                DebuggerCommand::BreakPoint(arg_opt) => {
                    // ADDRESS
                    if arg_opt.is_some() {
                        let arg = arg_opt.unwrap();
                        self.breakpoint_solover(arg);
                    }else {
                        println!("Usage: b|break|breakpoint [address|func_name|line_number]");
                    }
                }
                DebuggerCommand::Quit => {
                    if self.inferior.is_some() {
                        self.inferior.as_mut().unwrap().kill();
                    }
                    return;
                }
            }
        }
    }

    fn breakpoint_solover(&mut self, arg: String) {
        let mut address_opt : Option<usize> = None;

        if arg.starts_with("*") {
            address_opt = Debugger::parse_address(&arg[1..]);
        }else if  arg.parse::<usize>().is_ok() { // line number
            let line_number = arg.parse::<usize>().unwrap();
            address_opt = self.debug_data.get_addr_for_line(None, line_number);
        }else { // probably a function name
            address_opt = self.debug_data.get_addr_for_function(None, &arg);
        }

        let address : usize;
        match address_opt {
            Some(addr) => {
                println!("Set breakpoint {} at {:#x}", self.break_points.len(), addr);
                address = addr;
            },
            None => {
                println!("Can't set the breakpoint with format '{}'", arg);
                return;
            }
        }
        self.break_points.push(address.clone());
        if !self.inferior.is_none()  && !self.inferior.as_ref().unwrap().is_exited() {
            let orig_byte = self.inferior.as_mut().unwrap()
                .write_byte(address.clone(), 0xcc).ok().unwrap();
            self.brk_point_map.insert(address, orig_byte);
        }
    }

    fn parse_address(addr: &str) -> Option<usize> {
        let addr_without_0x = if addr.to_lowercase().starts_with("0x") {
            &addr[2..]
        } else {
            &addr
        };
        usize::from_str_radix(addr_without_0x, 16).ok()
    }

    fn print_condition(&self, status : Status) {
        match status {
            Status::Exited(exit_code) => {
                println!("Child exited (status {})", exit_code);
            }
            Status::Signaled(signal) => {
                println!("Child received signal {:?}", signal);
            }
            Status::Stopped(signal, rip) => {
                println!("Child stopped (signal {:?})", signal);
                match self.debug_data.get_line_from_addr(rip) {
                    Some(val) => {
                        println!("Stopped at  {}", val);
                    },
                    None => {
                        return;
                    }
                }
            }
        }
    }

    /// This function prompts the user to enter a command, and continues re-prompting until the user
    /// enters a valid command. It uses DebuggerCommand::from_tokens to do the command parsing.
    ///
    /// You don't need to read, understand, or modify this function.
    fn get_next_command(&mut self) -> DebuggerCommand {
        loop {
            // Print prompt and get next line of user input
            match self.readline.readline("(deet) ") {
                Err(ReadlineError::Interrupted) => {
                    // User pressed ctrl+c. We're going to ignore it
                    println!("Type \"quit\" to exit");
                }
                Err(ReadlineError::Eof) => {
                    // User pressed ctrl+d, which is the equivalent of "quit" for our purposes
                    return DebuggerCommand::Quit;
                }
                Err(err) => {
                    panic!("Unexpected I/O error: {:?}", err);
                }
                Ok(line) => {
                    if line.trim().len() == 0 {
                        continue;
                    }
                    self.readline.add_history_entry(line.as_str());
                    if let Err(err) = self.readline.save_history(&self.history_path) {
                        println!(
                            "Warning: failed to save history file at {}: {}",
                            self.history_path, err
                        );
                    }
                    let tokens: Vec<&str> = line.split_whitespace().collect();
                    if let Some(cmd) = DebuggerCommand::from_tokens(&tokens) {
                        return cmd;
                    } else {
                        println!("Unrecognized command.");
                    }
                }
            }
        }
    }
}
