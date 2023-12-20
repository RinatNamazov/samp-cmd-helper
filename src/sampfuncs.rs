/*****************************************************************************
 *
 *  PROJECT:        samp-cmd-helper
 *  LICENSE:        See LICENSE in the top level directory
 *  FILE:           sampfuncs.rs
 *  DESCRIPTION:    SAMPFUNCS functions
 *  COPYRIGHT:      (c) 2023 RINWARES <rinwares.com>
 *  AUTHOR:         Rinat Namazov <rinat.namazov@rinwares.com>
 *
 *****************************************************************************/

use std::error::Error;
use std::ffi::c_void;

use windows::core::{s, w};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

use crate::cppstd::{StdString, StdVector};
use crate::errors::FunctionNotFoundError;

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

pub unsafe fn initialize() -> Result<(), Box<dyn Error>> {
    let base_address = GetModuleHandleW(w!("SAMPFUNCS.asi"))?;

    GET_CHAT_COMMANDS = Some(std::mem::transmute(GetProcAddress(base_address, s!("?getChatCommands@SAMPFUNCS@@QAE?AV?$vector@UstCommandInfo@@V?$allocator@UstCommandInfo@@@std@@@std@@XZ")).ok_or(FunctionNotFoundError)?));
    GET_PLUGIN_NAME = Some(std::mem::transmute(GetProcAddress(base_address, s!("?getPluginName@SFPluginInfo@@QAE?AV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@XZ")).ok_or(FunctionNotFoundError)?));
    GET_SCM_THREAD_NAME = Some(std::mem::transmute(GetProcAddress(base_address, s!("?GetThreadName@CScriptThread@@QAE?AV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@XZ")).ok_or(FunctionNotFoundError)?));

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

    /*pub fn get_chat_commands_grouped(&self) -> HashMap<String, (CommandType, Vec<String>)> {
        let get_chat_commands = unsafe { GET_CHAT_COMMANDS.unwrap() };
        let sfcmds = get_chat_commands();

        let mut commands = HashMap::new();

        for cmd in &sfcmds {
            unsafe {
                let owner_name = match cmd.owner_type {
                    CommandType::SCRIPT => {
                        let script = &*(cmd.owner as *const ScmThread);
                        (self.get_scm_thread_name)(script)
                            .to_string()
                            .trim_end()
                            .to_string()
                            + ".cs"
                    }
                    CommandType::PLUGIN => {
                        let plugin = &*(cmd.owner as *const SfPluginInfo);
                        (self.get_plugin_name)(plugin).to_string()
                    }
                    _ => "unknown".to_string(),
                };

                commands
                    .entry(owner_name)
                    .or_insert((cmd.owner_type, Vec::new()))
                    .1
                    .push(cmd.name.to_string());
            }
        }

        commands
    }*/
}