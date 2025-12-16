pub fn get_env_var(key: &str) -> anyhow::Result<String> {
    get_optional_env_var(key)?.ok_or(anyhow::anyhow!(
        r#"environment variable "{}" is not set"#,
        key
    ))
}

pub fn get_optional_env_var(key: &str) -> anyhow::Result<Option<String>> {
    match std::env::var(key) {
        Ok(v) => Ok(Some(v)),
        Err(e) => match e {
            std::env::VarError::NotPresent => Ok(None),
            std::env::VarError::NotUnicode(_) => Err(anyhow::anyhow!(
                r#"environment variable "{}" is invalid unicode"#,
                key
            )),
        },
    }
}
