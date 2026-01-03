use super::ApprovedCmds;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub approved_commands: ApprovedCmds,
}
