pub enum DebuggerCommand {
    Quit,
    Run(Vec<String>),
    Continue,
    Backtrace,
    BreakPoint(Option<String>)
}

impl DebuggerCommand {
    pub fn from_tokens(tokens: &Vec<&str>) -> Option<DebuggerCommand> {
        match tokens[0] {
            "q" | "quit" => Some(DebuggerCommand::Quit),
            "r" | "run" => {
                let args = tokens[1..].to_vec();
                Some(DebuggerCommand::Run(
                    args.iter().map(|s| s.to_string()).collect(),
                ))
            },
            "c" | "cont" | "continue" => Some(DebuggerCommand::Continue),
            "bt" | "back" | "backtrace" => Some(DebuggerCommand::Backtrace),
            "b" | "break" | "breakpoint" =>{
                let mut  arg_1 : Option<String> = None;
                if tokens.len() > 1 {
                    arg_1 = Some(tokens[1].to_string());
                }
                Some(DebuggerCommand::BreakPoint(arg_1))
            },
            // Default case:
            _ => None,
        }
    }
}
