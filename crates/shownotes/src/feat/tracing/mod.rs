// Copyright (C) 2026 Jayson Lennon
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// 
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Tracing initialization for the shownotes application.
//!
//! This module provides utilities for setting up structured logging using
//! the `tracing` crate. It supports file-based logging with optional terminal
//! output.

use std::{env, fs::File, path::Path, sync::Arc};

use clap_verbosity_flag::{Verbosity, WarnLevel};
use error_stack::{Report, ResultExt};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use wherror::Error;

/// Error type returned when tracing subscriber initialization fails.
///
/// This error is used when the tracing system cannot be properly initialized,
/// such as when failing to open or configure log files.
#[derive(Debug, Error)]
#[error(debug)]
pub struct TracingInitError;

/// Initializes the tracing subscriber with the specified verbosity level.
///
/// This function sets up the global tracing subscriber for the application.
/// If the `RUST_LOG` environment variable is set, it takes precedence over
/// the verbosity parameter for filtering log output.
///
/// # Arguments
///
/// * `verbosity` - The verbosity level from CLI flags
/// * `file` - Optional path to the log file. If None, logs only to terminal
/// * `also_terminal` - If true and file is provided, also log to terminal
///
/// # Errors
///
/// Returns a `TracingInitError` if the log file cannot be opened or configured.
///
/// # Panics
///
/// Panics if called more than once or if another tracer has already been initialized.
pub fn init<P>(
    verbosity: Verbosity<WarnLevel>,
    file: Option<P>,
    also_terminal: bool,
) -> Result<(), Report<TracingInitError>>
where
    P: AsRef<Path>,
{
    let filter = match env::var("RUST_LOG") {
        // Use RUST_LOG if found
        Ok(filter_str) => filter_str,
        // Otherwise use this fallback based on verbosity
        Err(_) => format!("shownotes={verbosity}"),
    };

    match file {
        Some(path) => {
            let path = path.as_ref();
            let logfile = File::options()
                .create(true)
                .append(true)
                .open(path)
                .change_context(TracingInitError)
                .attach_with(|| format!("failed to open file '{}' for tracing", path.display()))?;

            let file_layer: Box<dyn Layer<_> + Send + Sync + 'static> =
                tracing_subscriber::fmt::layer()
                    .with_file(true)
                    .with_line_number(true)
                    .with_target(true)
                    .with_writer(Arc::new(logfile))
                    .with_filter(EnvFilter::new(filter.clone()))
                    .boxed();

            if also_terminal {
                let terminal_layer = tracing_subscriber::fmt::layer()
                    .with_file(true)
                    .with_line_number(true)
                    .with_target(true)
                    .with_filter(EnvFilter::new(filter));

                tracing_subscriber::registry()
                    .with(file_layer)
                    .with(terminal_layer)
                    .init();
            } else {
                tracing_subscriber::registry().with(file_layer).init();
            }
        }
        None => {
            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer().with_filter(EnvFilter::new(filter)))
                .init();
        }
    }

    tracing::info!("");
    tracing::info!("--- new session started ---");
    tracing::info!("");

    Ok(())
}
