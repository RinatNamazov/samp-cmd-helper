/*****************************************************************************
 *
 *  PROJECT:        samp-cmd-helper
 *  LICENSE:        See LICENSE in the top level directory
 *  FILE:           errors.rs
 *  DESCRIPTION:    Errors
 *  COPYRIGHT:      (c) 2023 RINWARES <rinwares.com>
 *  AUTHOR:         Rinat Namazov <rinat.namazov@rinwares.com>
 *
 *****************************************************************************/

use std::error::Error;
use std::fmt;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FunctionNotFoundError;

impl fmt::Display for FunctionNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GetProcAddress failed")
    }
}

impl Error for FunctionNotFoundError {}