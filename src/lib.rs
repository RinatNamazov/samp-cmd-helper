/*****************************************************************************
 *
 *  PROJECT:        samp-cmd-helper
 *  LICENSE:        See LICENSE in the top level directory
 *  FILE:           lib.rs
 *  DESCRIPTION:    DllMain
 *  COPYRIGHT:      (c) 2023 RINWARES <rinwares.com>
 *  AUTHOR:         Rinat Namazov <rinat.namazov@rinwares.com>
 *
 *****************************************************************************/

use windows::Win32::{
    Foundation::{BOOL, HMODULE, TRUE},
    System::{LibraryLoader::DisableThreadLibraryCalls, SystemServices::DLL_PROCESS_ATTACH},
};
#[cfg(debug_assertions)]
use windows::Win32::System::Console::AllocConsole;

mod gta;
mod plugin;
mod samp;
mod utils;
mod sampfuncs;
mod errors;
mod cppstd;

fn main_thread() {
    #[cfg(debug_assertions)]
    unsafe {
        AllocConsole().unwrap();
    }

    plugin::initialize();
}

#[no_mangle]
extern "stdcall" fn DllMain(instance: HMODULE, reason: u32, _reserved: *mut ()) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        unsafe {
            DisableThreadLibraryCalls(instance).unwrap();
        }
        std::thread::spawn(main_thread);
    }
    TRUE
}
