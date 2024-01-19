/*****************************************************************************
 *
 *  PROJECT:        samp-cmd-helper
 *  LICENSE:        See LICENSE in the top level directory
 *  FILE:           moonloader.rs
 *  DESCRIPTION:    MoonLoader Lua API functions hooks
 *  COPYRIGHT:      (c) 2024 RINWARES <rinwares.com>
 *  AUTHOR:         Rinat Namazov <rinat.namazov@rinwares.com>
 *
 *****************************************************************************/

use std::ffi::{c_char, CStr};
use std::path::Path;

use windows::{core::w, Win32::System::LibraryLoader::GetModuleHandleW};

use crate::errors::Error;
use crate::plugin::Plugin;
use crate::utils;

// libc
extern "C" {
    fn wcslen(buf: *const u16) -> usize;
}

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
pub enum Version {
    V0265BetaArchive,
    V0265BetaInstaller,
    V0270Preview3,
}

pub fn get_version(base_address: usize) -> Result<Version, Error> {
    match utils::get_entry_point(base_address) {
        0x13D2CF => Ok(Version::V0265BetaArchive),
        0x13C2EE => Ok(Version::V0265BetaInstaller),
        0x13F632 => Ok(Version::V0270Preview3),
        ep => Err(Error::IncompatibleMoonLoaderVersion(ep)),
    }
}

struct MoonLoaderHooks {
    name_offset: usize,
    orig_samp_register_chat_command:
        unsafe extern "C" fn(usize, *const c_char, u32, u32, u32, u32) -> u8,
    orig_samp_unregister_chat_command: unsafe extern "C" fn(usize, *const c_char) -> u8,
}

static mut MOONLOADER_HOOKS: Option<MoonLoaderHooks> = None;

impl MoonLoaderHooks {
    pub fn new() -> Result<Self, Error> {
        unsafe {
            let base_address = GetModuleHandleW(w!("MoonLoader.asi"))?.0 as usize;

            match get_version(base_address)? {
                Version::V0265BetaArchive => Ok(Self {
                    name_offset: 0x18,
                    orig_samp_register_chat_command: utils::replace_data_and_return_original(
                        base_address + 0xF4438 + 0x4,
                        Self::hk_orig_samp_register_chat_command,
                    ),
                    orig_samp_unregister_chat_command: utils::replace_data_and_return_original(
                        base_address + 0xF44FE + 0x4,
                        Self::hk_orig_samp_unregister_chat_command,
                    ),
                }),
                Version::V0265BetaInstaller => Ok(Self {
                    name_offset: 0x18,
                    orig_samp_register_chat_command: utils::replace_data_and_return_original(
                        base_address + 0xF3918 + 0x4,
                        Self::hk_orig_samp_register_chat_command,
                    ),
                    orig_samp_unregister_chat_command: utils::replace_data_and_return_original(
                        base_address + 0xF39DE + 0x4,
                        Self::hk_orig_samp_unregister_chat_command,
                    ),
                }),
                Version::V0270Preview3 => Ok(Self {
                    name_offset: 0x34,
                    orig_samp_register_chat_command: utils::replace_data_and_return_original(
                        base_address + 0xDF0A4 + 0x1,
                        Self::hk_orig_samp_register_chat_command,
                    ),
                    orig_samp_unregister_chat_command: utils::replace_data_and_return_original(
                        base_address + 0xDF14C + 0x1,
                        Self::hk_orig_samp_unregister_chat_command,
                    ),
                }),
            }
        }
    }

    pub unsafe fn get_script_name_from_userdata(&self, userdata: usize) -> String {
        // Userdata is _G[".moonloader.this_script"]
        let name = *((userdata + self.name_offset) as *const *const u16);
        let name = String::from_utf16_lossy(std::slice::from_raw_parts(name, wcslen(name)));
        if let Some(name) = Path::new(&name).file_name() {
            name.to_string_lossy().to_string()
        } else {
            "unknown".to_string()
        }
    }

    unsafe extern "C" fn hk_orig_samp_register_chat_command(
        userdata: usize,
        cmd: *const c_char,
        a3: u32,
        a4: u32,
        a5: u32,
        a6: u32,
    ) -> u8 {
        let mh = MOONLOADER_HOOKS.as_ref().unwrap();

        if let Ok(cmd) = CStr::from_ptr(cmd).to_str() {
            let script_name = mh.get_script_name_from_userdata(userdata);
            Plugin::get().add_lua_command(script_name, cmd);
        }

        (mh.orig_samp_register_chat_command)(userdata, cmd, a3, a4, a5, a6)
    }

    unsafe extern "C" fn hk_orig_samp_unregister_chat_command(
        userdata: usize,
        cmd: *const c_char,
    ) -> u8 {
        let mh = MOONLOADER_HOOKS.as_ref().unwrap();

        if let Ok(cmd) = CStr::from_ptr(cmd).to_str() {
            let script_name = mh.get_script_name_from_userdata(userdata);
            Plugin::get().remove_lua_command(&script_name, cmd);
        }

        (mh.orig_samp_unregister_chat_command)(userdata, cmd)
    }
}

pub fn is_initialized() -> bool {
    unsafe { MOONLOADER_HOOKS.is_some() }
}

pub fn initialize() -> Result<(), Error> {
    match MoonLoaderHooks::new() {
        Ok(v) => unsafe {
            MOONLOADER_HOOKS = Some(v);
            Ok(())
        },
        Err(e) => Err(e),
    }
}
