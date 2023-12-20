/*****************************************************************************
 *
 *  PROJECT:        samp-cmd-helper
 *  LICENSE:        See LICENSE in the top level directory
 *  FILE:           utils.rs
 *  DESCRIPTION:    Utils
 *  COPYRIGHT:      (c) 2023 RINWARES <rinwares.com>
 *  AUTHOR:         Rinat Namazov <rinat.namazov@rinwares.com>
 *
 *****************************************************************************/

use core::ffi::c_void;

use windows::Win32::{
    Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, MODULEENTRY32W,
            TH32CS_SNAPMODULE,
        },
        Memory::{PAGE_EXECUTE_READWRITE, VirtualProtect},
        Threading::GetCurrentProcessId,
    },
};
use windows::Win32::System::Diagnostics::Debug::IMAGE_NT_HEADERS32;
use windows::Win32::System::SystemServices::IMAGE_DOS_HEADER;

pub fn get_entry_point(base_address: usize) -> u32 {
    unsafe {
        let dos_header = *(base_address as *const IMAGE_DOS_HEADER);
        let nt_headers =
            *((base_address + (dos_header.e_lfanew as usize)) as *const IMAGE_NT_HEADERS32);

        nt_headers.OptionalHeader.AddressOfEntryPoint
    }
}

pub unsafe fn patch_pointer(address: usize, value: usize) {
    let address = address as *const c_void;
    let size = std::mem::size_of::<usize>();
    let mut vp = PAGE_EXECUTE_READWRITE;
    VirtualProtect(address, size, vp, &mut vp).unwrap();
    *(address as *mut usize) = value;
    VirtualProtect(address, size, vp, &mut vp).unwrap();
}

pub unsafe fn patch_call_address(address: usize, value: usize) {
    patch_pointer(address + 1, value - address - 1 - 4);
}

pub unsafe fn extract_call_target_address(address: usize) -> usize {
    let relative = *((address + 1) as *const usize);
    address + relative + 1 + 4
}

pub fn find_module_name_that_owns_address_list(
    addresses: &[*const c_void],
) -> Option<Vec<Option<String>>> {
    let snapshot =
        unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE, GetCurrentProcessId()) }.unwrap();
    if snapshot == INVALID_HANDLE_VALUE {
        return None;
    }

    let mut module_entry32 = MODULEENTRY32W::default();
    module_entry32.dwSize = std::mem::size_of::<MODULEENTRY32W>() as u32;

    if unsafe { Module32FirstW(snapshot, &mut module_entry32) }.is_err() {
        unsafe {
            CloseHandle(snapshot).unwrap();
        }
        return None;
    }

    let mut module_names = vec![None; addresses.len()];

    loop {
        for (index, &address) in addresses.iter().enumerate() {
            let module_name = &mut module_names[index];
            if module_name.is_none() {
                let address = address as *const u8;
                if address > module_entry32.modBaseAddr && address < unsafe { module_entry32.modBaseAddr.add(module_entry32.modBaseSize as usize) } {
                    *module_name = Some(u16_slice_to_string(&module_entry32.szModule));
                }
            }
        }

        if unsafe { Module32NextW(snapshot, &mut module_entry32) }.is_err() {
            break;
        }
    }

    unsafe {
        CloseHandle(snapshot).unwrap();
    }

    Some(module_names)
}

pub fn u16_slice_to_string(slice: &[u16]) -> String {
    slice
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| char::from_u32(c as u32).unwrap_or('?'))
        .collect()
}
