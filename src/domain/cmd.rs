use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct CmdPattern {
    binary: String,
    first_arg: Option<String>,
}

impl FromStr for CmdPattern {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cmd = match shlex::split(s) {
            Some(c) if !c.is_empty() => c,
            _ => return Err("command is empty"),
        };

        Ok(Self {
            binary: cmd[0].clone(),
            first_arg: if cmd.len() > 1 {
                Some(cmd[1].clone())
            } else {
                None
            },
        })
    }
}

impl Display for CmdPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.first_arg {
            Some(a) => write!(f, "{} {} .*", self.binary, a),
            None => write!(f, "{} .*", self.binary),
        }
    }
}
