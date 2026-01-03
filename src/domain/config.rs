use super::ApprovedCmds;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    approved_commands: ApprovedCmds,
}
