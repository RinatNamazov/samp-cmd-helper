/*****************************************************************************
 *
 *  PROJECT:        samp-cmd-helper
 *  LICENSE:        See LICENSE in the top level directory
 *  FILE:           lib.rs
 *  DESCRIPTION:    DllMain
 *  COPYRIGHT:      (c) 2023-2024 RINWARES <rinwares.com>
 *  AUTHOR:         Rinat Namazov <rinat.namazov@rinwares.com>
 *
 *****************************************************************************/

#[cfg(debug_assertions)]
use windows::Win32::System::Console::AllocConsole;
use windows::Win32::{
    Foundation::{BOOL, FALSE, HMODULE, TRUE},
    System::{LibraryLoader::DisableThreadLibraryCalls, SystemServices::DLL_PROCESS_ATTACH},
};

mod cmd_storage;
mod cppstd;
mod errors;
mod gta;
mod gui;
mod plugin;
mod samp;
mod sampfuncs;
mod utils;

#[no_mangle]
extern "stdcall" fn DllMain(instance: HMODULE, reason: u32, _reserved: *mut ()) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        unsafe {
            #[cfg(debug_assertions)]
            AllocConsole().unwrap();

            if let Err(e) = plugin::initialize() {
                eprintln!("plugin::initialize: {}", e);
                return FALSE;
            }

            // We intentionally do not check the result of DisableThreadLibraryCalls,
            // as it is not crucial for our functionality. This is done to prevent
            // Windows from invoking our DllMain during thread creation/destruction,
            // which is unnecessary for our specific requirements.
            let _ = DisableThreadLibraryCalls(instance);
        }
    }
    TRUE
}
