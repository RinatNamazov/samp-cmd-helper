/*****************************************************************************
 *
 *  PROJECT:        samp-cmd-helper
 *  LICENSE:        See LICENSE in the top level directory
 *  FILE:           errors.rs
 *  DESCRIPTION:    Errors
 *  COPYRIGHT:      (c) 2023-2024 RINWARES <rinwares.com>
 *  AUTHOR:         Rinat Namazov <rinat.namazov@rinwares.com>
 *
 *****************************************************************************/

use std::fmt;
use windows::core::Error as WindowsError;

#[derive(Debug)]
pub enum Error {
    WinApiError(WindowsError),
    FunctionNotFound(String),
    MaybeInvalidGameOrPluginConflicting,
    SampNotLoaded(WindowsError),
    IncompatibleSampVersion,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::WinApiError(e) => write!(f, "WinAPI: {}", e),
            Error::FunctionNotFound(symbol) => {
                write!(f, "GetProcAddress failed for symbol: {}", symbol)
            }
            Error::MaybeInvalidGameOrPluginConflicting => {
                write!(f, "Maybe invalid game or conflicting plugin")
            }
            Error::SampNotLoaded(e) => write!(f, "Library 'samp.dll' not found. WinAPI: {}", e),
            Error::IncompatibleSampVersion => write!(f, "Incompatible SA-MP version"),
        }
    }
}

impl std::error::Error for Error {}

impl From<WindowsError> for Error {
    fn from(e: WindowsError) -> Self {
        Error::WinApiError(e)
    }
}
