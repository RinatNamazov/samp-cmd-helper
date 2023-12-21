/*****************************************************************************
 *
 *  PROJECT:        samp-cmd-helper
 *  LICENSE:        See LICENSE in the top level directory
 *  FILE:           samp.rs
 *  DESCRIPTION:    SA-MP misc functions
 *  COPYRIGHT:      (c) 2023 RINWARES <rinwares.com>
 *  AUTHOR:         Rinat Namazov <rinat.namazov@rinwares.com>
 *
 *****************************************************************************/

use std::ffi::{c_char, c_void, CStr, CString};

use windows::Win32::Foundation::BOOL;
use windows::Win32::Graphics::Direct3D9::IDirect3DDevice9;

use crate::utils::get_entry_point;

static mut INPUT: Option<*mut Input> = None;
static mut DXUT_EDIT_BOX_GET_TEXT: Option<DxutEditBoxGetText> = None;
static mut DXUT_EDIT_BOX_SET_TEXT: Option<DxutEditBoxSetText> = None;

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
pub enum Version {
    V037R1,
    V037R2,
    V037R3,
    V037R3_1,
    V037R4,
    V037R4_2,
    V037R5,
    V03DLR1,
}

pub fn get_version(base_address: usize) -> Option<Version> {
    match get_entry_point(base_address) {
        0x31DF13 => Some(Version::V037R1),
        0x3195DD => Some(Version::V037R2),
        0xCC490 => Some(Version::V037R3),
        0xCC4D0 => Some(Version::V037R3_1),
        0xCBCD0 => Some(Version::V037R4),
        0xCBCB0 => Some(Version::V037R4_2),
        0xCBC90 => Some(Version::V037R5),
        0xFDB60 => Some(Version::V03DLR1),
        _ => None,
    }
}

pub fn initialize(base_address: usize, version: Version) {
    unsafe {
        INPUT = Some(*((base_address + get_input_offset(version)) as *mut *mut Input));
        DXUT_EDIT_BOX_GET_TEXT = Some(std::mem::transmute(base_address + get_offset_of_dxut_edit_box_get_text(version)));
        DXUT_EDIT_BOX_SET_TEXT = Some(std::mem::transmute(base_address + get_offset_of_dxut_edit_box_set_text(version)));
    }
}

fn get_input_offset(version: Version) -> usize {
    match version {
        Version::V037R1 => 0x21A0E8,
        Version::V037R2 => 0x21A0F0,
        Version::V037R3 | Version::V037R3_1 => 0x26E8CC,
        Version::V037R4 | Version::V037R4_2 => 0x26E9FC,
        Version::V037R5 => 0x26EB84,
        Version::V03DLR1 => 0x2ACA14,
    }
}

fn get_offset_of_dxut_edit_box_get_text(version: Version) -> usize {
    match version {
        Version::V037R1 => 0x81030,
        Version::V037R2 => 0x810D0,
        Version::V037R3 | Version::V037R3_1 => 0x84F40,
        Version::V037R4 => 0x85680,
        Version::V037R4_2 => 0x856B0,
        Version::V037R5 => 0x85650,
        Version::V03DLR1 => 0x850D0,
    }
}

fn get_offset_of_dxut_edit_box_set_text(version: Version) -> usize {
    match version {
        Version::V037R1 => 0x80F60,
        Version::V037R2 => 0x81000,
        Version::V037R3 | Version::V037R3_1 => 0x84E70,
        Version::V037R4 => 0x855B0,
        Version::V037R4_2 => 0x855E0,
        Version::V037R5 => 0x85580,
        Version::V03DLR1 => 0x85000,
    }
}

pub const MAX_CLIENT_CMDS: usize = 144;
pub const MAX_CMD_LENGTH: usize = 32;
pub const MAX_CHAT_INPUT: usize = 128;
pub const MAX_RECALL_HISTORY: usize = 10;

#[repr(C, align(1))]
pub struct Input {
    pub device: *const IDirect3DDevice9,
    pub game_ui: *mut c_void,
    pub edit_box: *mut DXUTEditBox,
    pub command_proc: [*const c_void; MAX_CLIENT_CMDS],
    pub command_name: [[u8; MAX_CMD_LENGTH + 1]; MAX_CLIENT_CMDS],
    pub command_count: i32,
    pub enabled: BOOL,
    pub input: [u8; MAX_CHAT_INPUT + 1],
    pub recall_buffer: [[u8; MAX_CHAT_INPUT + 1]; MAX_RECALL_HISTORY],
    pub current_buffer: [u8; MAX_CHAT_INPUT + 1],
    pub current_recall: i32,
    pub total_recall: i32,
    pub default_proc: *const u8,
}

impl Input {
    pub fn get<'a>() -> Option<&'a mut Input> {
        unsafe {
            match INPUT {
                Some(v) => Some(&mut *v),
                None => None
            }
        }
    }

    pub fn edit_box(&self) -> &mut DXUTEditBox {
        unsafe { &mut *self.edit_box }
    }
}

#[repr(C, align(1))]
pub struct DXUTEditBox {
    _unnecessary: [u8; 8],
    pub position: [i32; 2],
    pub width: i32,
    pub height: i32,
}

type DxutEditBoxGetText = extern "thiscall" fn(*const DXUTEditBox) -> *const c_char;
type DxutEditBoxSetText = extern "thiscall" fn(*mut DXUTEditBox, *const c_char, bool);

impl DXUTEditBox {
    pub fn set_text_raw(&mut self, text: *const c_char) {
        let func = unsafe { DXUT_EDIT_BOX_SET_TEXT.unwrap() };
        func(self as *mut Self, text, false)
    }

    pub fn get_text<'a>(&self) -> String {
        unsafe {
            let func = DXUT_EDIT_BOX_GET_TEXT.unwrap();
            let c_str = func(self as *const Self);
            CStr::from_ptr(c_str).to_string_lossy().to_string()
        }
    }

    pub fn set_text(&mut self, text: &str) {
        let c_str = CString::new(text).unwrap();
        self.set_text_raw(c_str.as_ptr());
    }
}