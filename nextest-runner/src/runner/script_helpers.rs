// Copyright (c) The nextest Contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{errors::SetupScriptOutputError, reporter::events::SetupScriptEnvMap};
use camino::Utf8Path;
use std::{collections::BTreeMap, sync::Arc};
use tokio::io::{AsyncBufReadExt, BufReader};

/// Parses an environment file generated by a setup script.
pub(super) async fn parse_env_file(
    env_path: &Utf8Path,
) -> Result<SetupScriptEnvMap, SetupScriptOutputError> {
    let mut env_map = BTreeMap::new();
    let f = tokio::fs::File::open(env_path).await.map_err(|error| {
        SetupScriptOutputError::EnvFileOpen {
            path: env_path.to_owned(),
            error: Arc::new(error),
        }
    })?;
    let reader = BufReader::new(f);
    let mut lines = reader.lines();
    loop {
        let line =
            lines
                .next_line()
                .await
                .map_err(|error| SetupScriptOutputError::EnvFileRead {
                    path: env_path.to_owned(),
                    error: Arc::new(error),
                })?;
        let Some(line) = line else { break };

        // Split this line into key and value.
        let (key, value) = match line.split_once('=') {
            Some((key, value)) => (key, value),
            None => {
                return Err(SetupScriptOutputError::EnvFileParse {
                    path: env_path.to_owned(),
                    line: line.to_owned(),
                })
            }
        };

        // Ban keys starting with `NEXTEST`.
        if key.starts_with("NEXTEST") {
            return Err(SetupScriptOutputError::EnvFileReservedKey {
                key: key.to_owned(),
            });
        }

        env_map.insert(key.to_owned(), value.to_owned());
    }

    Ok(SetupScriptEnvMap { env_map })
}
