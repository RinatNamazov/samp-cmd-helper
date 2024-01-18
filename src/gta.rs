/*****************************************************************************
 *
 *  PROJECT:        samp-cmd-helper
 *  LICENSE:        See LICENSE in the top level directory
 *  FILE:           gta.rs
 *  DESCRIPTION:    GTA:SA
 *  COPYRIGHT:      (c) 2023 RINWARES <rinwares.com>
 *  AUTHOR:         Rinat Namazov <rinat.namazov@rinwares.com>
 *
 *****************************************************************************/

use std::ffi::c_void;
use windows::Win32::{
    Foundation::HWND,
    Graphics::Direct3D9::IDirect3DDevice9,
};

pub fn get_window_handle() -> HWND {
    unsafe { **(0xC17054 as *const *const HWND) }
}

pub fn get_d3d9_device() -> IDirect3DDevice9 {
    unsafe {
        windows::core::Interface::from_raw(*(0xC97C28 as *const *mut c_void))
    }
}

pub fn is_gta_menu_active() -> bool {
    unsafe { *(0xBA67A4 as *const bool) }
}