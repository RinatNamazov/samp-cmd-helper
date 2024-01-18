/*****************************************************************************
 *
 *  PROJECT:        samp-cmd-helper
 *  LICENSE:        See LICENSE in the top level directory
 *  FILE:           sampfuncs.rs
 *  DESCRIPTION:    SAMPFUNCS functions
 *  COPYRIGHT:      (c) 2023-2024 RINWARES <rinwares.com>
 *  AUTHOR:         Rinat Namazov <rinat.namazov@rinwares.com>
 *
 *****************************************************************************/

use std::ffi::c_void;

use windows::core::{s, w};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

use crate::cppstd::{StdString, StdVector};
use crate::errors::Error;

#[repr(C)]
pub struct CommandInfo {
    pub name: StdString,
    pub owner_type: CommandType,
    owner: *const c_void,
}

pub enum CmdOwner<'a> {
    Nope,
    Script(&'a ScmThread),
    Plugin(&'a SfPluginInfo),
}

impl CommandInfo {
    pub fn owner(&self) -> CmdOwner {
        match self.owner_type {
            CommandType::SCRIPT => {
                let script = unsafe { &*(self.owner as *const ScmThread) };
                CmdOwner::Script(script)
            }
            CommandType::PLUGIN => {
                let plugin = unsafe { &*(self.owner as *const SfPluginInfo) };
                CmdOwner::Plugin(plugin)
            }
            _ => CmdOwner::Nope,
        }
    }
}

#[repr(C)]
pub struct ScmThread {
    next: *const ScmThread,
    prev: *const ScmThread,
    thread_name: [u8; 8],
    // ...
}

impl ScmThread {
    pub fn thread_name(&self) -> String {
        let get_scm_thread_name = unsafe { GET_SCM_THREAD_NAME.unwrap() };
        get_scm_thread_name(self).to_string()
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(i32)]
pub enum CommandType {
    NOPE,
    SCRIPT,
    PLUGIN,
}

#[repr(C)]
pub struct SfPluginInfo {
    handle: usize,
    name: StdString,
}

impl SfPluginInfo {
    pub fn plugin_name(&self) -> String {
        let get_plugin_name = unsafe { GET_PLUGIN_NAME.unwrap() };
        get_plugin_name(self).to_string()
    }
}

static mut INITIALIZED: bool = false;

static mut GET_CHAT_COMMANDS: Option<extern "thiscall" fn() -> StdVector<CommandInfo>> = None;
static mut GET_PLUGIN_NAME: Option<extern "thiscall" fn(*const SfPluginInfo) -> StdString> = None;
static mut GET_SCM_THREAD_NAME: Option<extern "thiscall" fn(*const ScmThread) -> StdString> = None;

macro_rules! def_fn {
    ($handle:ident, $var:ident, $symbol:literal) => {
        $var = Some(std::mem::transmute(GetProcAddress($handle, s!($symbol)).ok_or(Error::FunctionNotFound($symbol.to_string()))?));
    };
}

pub unsafe fn initialize() -> Result<(), Error> {
    let handle = GetModuleHandleW(w!("SAMPFUNCS.asi"))?;

    def_fn!(handle, GET_CHAT_COMMANDS, "?getChatCommands@SAMPFUNCS@@QAE?AV?$vector@UstCommandInfo@@V?$allocator@UstCommandInfo@@@std@@@std@@XZ");
    def_fn!(handle, GET_PLUGIN_NAME, "?getPluginName@SFPluginInfo@@QAE?AV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@XZ");
    def_fn!(handle, GET_SCM_THREAD_NAME, "?GetThreadName@CScriptThread@@QAE?AV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@XZ");

    INITIALIZED = true;
    Ok(())
}

pub fn is_initialized() -> bool {
    unsafe { INITIALIZED }
}

pub struct SampFuncs {}

impl SampFuncs {
    pub fn get_chat_commands() -> StdVector<CommandInfo> {
        let get_chat_commands = unsafe { GET_CHAT_COMMANDS.unwrap() };
        get_chat_commands()
    }
}