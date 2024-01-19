/*****************************************************************************
 *
 *  PROJECT:        samp-cmd-helper
 *  LICENSE:        See LICENSE in the top level directory
 *  FILE:           plugin.rs
 *  DESCRIPTION:    Plugin
 *  COPYRIGHT:      (c) 2023-2024 RINWARES <rinwares.com>
 *  AUTHOR:         Rinat Namazov <rinat.namazov@rinwares.com>
 *
 *****************************************************************************/

use std::cell::OnceCell;
use std::collections::HashMap;
use std::ffi::CStr;
use std::time::{Duration, SystemTime};

use egui_d3d9::EguiDx9;
use vmt_hook::VTableHook;
use windows::{
    core::{w, HRESULT},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::{
            Direct3D9::{IDirect3DDevice9, D3DPRESENT_PARAMETERS},
            Gdi::RGNDATA,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CallWindowProcA, SetWindowLongPtrA, GWLP_WNDPROC, WM_LBUTTONDOWN, WNDPROC,
        },
    },
};

use crate::cmd_storage::{cmd_with_prefix, Categories, Category, CategoryKey, ModuleMap};
use crate::errors::Error;
use crate::gui::Ui;
use crate::sampfuncs::{CmdOwner, CommandType};
use crate::{gta, samp, sampfuncs, utils};

type FnPresent = extern "stdcall" fn(
    IDirect3DDevice9,
    *const RECT,
    *const RECT,
    HWND,
    *const RGNDATA,
) -> HRESULT;

type FnReset = extern "stdcall" fn(IDirect3DDevice9, *const D3DPRESENT_PARAMETERS) -> HRESULT;

enum InitState {
    BeforeSampInit,
    AfterSampInit,
    Initialized,
    Nothing,
}

static mut FUNC_GTA_DEFINED_STATE: Option<unsafe extern "C" fn()> = None;

static mut PLUGIN: Option<Plugin> = None;

pub struct Plugin {
    d3d9_hook: Option<VTableHook<IDirect3DDevice9>>,
    gui: Option<EguiDx9<Ui>>,
    commands: Categories,
    original_wnd_proc: Option<WNDPROC>,
    original_reset: Option<FnReset>,
    original_present: Option<FnPresent>,
    samp_base_address: usize,
    samp_version: samp::Version,
}

impl Plugin {
    pub fn new(samp_base_address: usize, samp_version: samp::Version) -> Self {
        Self {
            d3d9_hook: None,
            gui: None,
            commands: Categories {
                order: [CategoryKey::Samp, CategoryKey::SfPlugin, CategoryKey::Cleo],
                samp: Category::new("SA-MP".to_string()),
                sf: Category::new("SF".to_string()),
                cleo: Category::new("CLEO".to_string()),
            },
            original_wnd_proc: None,
            original_reset: None,
            original_present: None,
            samp_base_address,
            samp_version,
        }
    }

    pub fn get<'a>() -> &'a mut Plugin {
        unsafe {
            if cfg!(debug_assertions) {
                PLUGIN.as_mut().unwrap()
            } else {
                PLUGIN.as_mut().unwrap_unchecked()
            }
        }
    }

    pub fn post_initialize(&mut self) {
        unsafe {
            self.install_wnd_proc();
            self.install_d3d9_hooks();
            self.init_ui();
        }
    }

    pub fn commands(&self) -> &Categories {
        &self.commands
    }

    pub fn parse_commands(&mut self) {
        // Todo: Prefer placing hooks on command registration and removal rather than parsing them once.

        let samp_cmds: HashMap<String, Vec<String>> = self.get_samp_commands_grouped_by_module();
        let samp_modules = samp_cmds
            .into_iter()
            .map(|(module, cmds)| {
                (
                    module,
                    cmds.iter()
                        .map(|cmd| (cmd_with_prefix(cmd), String::default()))
                        .collect(),
                )
            })
            .collect();
        let samp = &mut self.commands.samp;
        samp.modules = samp_modules;
        samp.is_visible = true;

        if let Some(sf_cmds) = self.get_sampfuncs_commands_grouped() {
            let mut sf_modules = ModuleMap::new();
            let mut cleo_modules = ModuleMap::new();

            fn convert(modules: &mut ModuleMap, module: String, cmds: Vec<String>) {
                modules.entry(module).or_insert(
                    cmds.iter()
                        .map(|cmd| (cmd_with_prefix(cmd), String::default()))
                        .collect(),
                );
            }

            for (module, v) in sf_cmds {
                match v.0 {
                    CommandType::PLUGIN => convert(&mut sf_modules, module, v.1),
                    CommandType::SCRIPT => convert(&mut cleo_modules, module, v.1),
                    CommandType::NOPE => {}
                }
            }

            if !sf_modules.is_empty() {
                let sf = &mut self.commands.sf;
                sf.modules = sf_modules;
                sf.is_visible = true;
            }

            if !cleo_modules.is_empty() {
                let cleo = &mut self.commands.cleo;
                cleo.modules = cleo_modules;
                cleo.is_visible = true;
            }
        }
    }

    unsafe fn install_wnd_proc(&mut self) {
        let window = gta::get_window_handle();

        let old_proc = SetWindowLongPtrA(window, GWLP_WNDPROC, Self::hk_wnd_proc as i32);
        self.original_wnd_proc = Some(std::mem::transmute(old_proc));
    }

    unsafe fn install_d3d9_hooks(&mut self) {
        let hook = VTableHook::with_count(gta::get_d3d9_device(), 119);

        self.original_reset = Some(std::mem::transmute(hook.get_original_method(16)));
        self.original_present = Some(std::mem::transmute(hook.get_original_method(17)));

        hook.replace_method(16, Self::hk_reset as usize);
        hook.replace_method(17, Self::hk_present as usize);

        self.d3d9_hook = Some(hook);
    }

    fn init_ui(&mut self) {
        if let Some(device_hook) = &self.d3d9_hook {
            let gui = EguiDx9::<Ui>::init(
                device_hook.object(),
                gta::get_window_handle(),
                Ui::render_ui,
                Ui::new(),
                true,
            );

            Ui::init_style(gui.ctx());

            self.gui = Some(gui);
        }
    }

    unsafe extern "stdcall" fn hk_reset(
        device: IDirect3DDevice9,
        presentation_parameters: *const D3DPRESENT_PARAMETERS,
    ) -> HRESULT {
        let plugin = Plugin::get();
        let gui = plugin.gui.as_mut().unwrap_unchecked();
        gui.pre_reset();

        let original_reset = plugin.original_reset.unwrap_unchecked();
        original_reset(device, presentation_parameters)
    }

    unsafe extern "stdcall" fn hk_present(
        device: IDirect3DDevice9,
        source_rect: *const RECT,
        dest_rect: *const RECT,
        dest_window_override: HWND,
        dirty_region: *const RGNDATA,
    ) -> HRESULT {
        let plugin = Plugin::get();
        let gui = plugin.gui.as_mut().unwrap_unchecked();
        gui.present(&device);

        let original_present = plugin.original_present.unwrap_unchecked();
        original_present(
            device,
            source_rect,
            dest_rect,
            dest_window_override,
            dirty_region,
        )
    }

    unsafe extern "stdcall" fn hk_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let plugin = Plugin::get();
        let gui = plugin.gui.as_mut().unwrap_unchecked();
        gui.wnd_proc(msg, wparam, lparam);

        if msg == WM_LBUTTONDOWN && gui.ctx().wants_pointer_input() {
            // To prevent the chat from closing when clicking on our interface.
            LRESULT(1)
        } else {
            CallWindowProcA(
                plugin.original_wnd_proc.unwrap_unchecked(),
                hwnd,
                msg,
                wparam,
                lparam,
            )
        }
    }

    fn get_samp_commands_grouped_by_module(&self) -> HashMap<String, Vec<String>> {
        let input = samp::Input::get().unwrap();
        let cmd_count = input.command_count as usize;
        let mut module_commands = HashMap::new();

        if cmd_count > 0 {
            let addresses = input.command_proc[..cmd_count].to_vec();
            let module_names = utils::find_module_name_that_owns_address_list(&addresses).unwrap();

            for (i, module_name) in module_names.iter().enumerate() {
                let module_name = module_name.clone().unwrap_or("unknown".to_string());

                let cmd = if let Ok(cstr) = CStr::from_bytes_until_nul(&input.command_name[i]) {
                    cstr.to_string_lossy().to_string()
                } else {
                    "unknown".to_string()
                };

                module_commands
                    .entry(module_name)
                    .or_insert(Vec::new())
                    .push(cmd);
            }
        }

        module_commands
    }

    fn get_sampfuncs_commands_grouped(
        &self,
    ) -> Option<HashMap<String, (CommandType, Vec<String>)>> {
        if !sampfuncs::is_initialized() {
            return None;
        }

        let sf_cmds = sampfuncs::SampFuncs::get_chat_commands();
        let mut commands = HashMap::new();

        for cmd in &sf_cmds {
            let owner_name = match cmd.owner() {
                CmdOwner::Nope => "unknown".to_string(),
                CmdOwner::Script(s) => s.thread_name().trim_end().to_string() + ".cs",
                CmdOwner::Plugin(p) => p.plugin_name(),
            };

            commands
                .entry(owner_name)
                .or_insert((cmd.owner_type, Vec::new()))
                .1
                .push(cmd.name.to_string());
        }

        Some(commands)
    }
}

unsafe fn initialize_plugin() {
    static mut STATE: InitState = InitState::BeforeSampInit;

    match STATE {
        InitState::BeforeSampInit => {
            STATE = InitState::AfterSampInit;
        }
        InitState::AfterSampInit => {
            let plugin = Plugin::get();

            samp::initialize(plugin.samp_base_address, plugin.samp_version);

            // We can work without this module.
            if let Err(e) = sampfuncs::initialize() {
                eprintln!("sampfuncs::initialize: {}", e);
            }

            plugin.post_initialize();

            STATE = InitState::Initialized;
        }
        InitState::Initialized => {
            static mut TIME: OnceCell<SystemTime> = OnceCell::new();
            let time = TIME.get_or_init(|| SystemTime::now());

            // We wait for some time during which other plugins will most likely register their commands.
            if time.elapsed().unwrap() > Duration::from_secs(3) {
                let plugin = Plugin::get();
                plugin.parse_commands();

                STATE = InitState::Nothing;
            }
        }
        InitState::Nothing => {}
    }
}

// This function is called cyclically in the game.
unsafe extern "C" fn hk_defined_state() {
    initialize_plugin();
    FUNC_GTA_DEFINED_STATE.unwrap()();
}

pub fn initialize() -> Result<(), Error> {
    const ADDRESS_OF_CALL_DEFINED_STATE_IN_IDLE: usize = 0x53EA8E;

    let current_byte = unsafe { *(ADDRESS_OF_CALL_DEFINED_STATE_IN_IDLE as *const u8) };
    if current_byte != 0xE8
    /* call opcode */
    {
        return Err(Error::MaybeInvalidGameOrPluginConflicting);
    }

    let samp_base_address = match unsafe { GetModuleHandleW(w!("samp.dll")) } {
        Ok(handle) => handle.0 as usize,
        Err(e) => return Err(Error::SampNotLoaded(e)),
    };

    match samp::get_version(samp_base_address) {
        Some(samp_version) => unsafe {
            PLUGIN = Some(Plugin::new(samp_base_address, samp_version));

            FUNC_GTA_DEFINED_STATE = Some(std::mem::transmute(utils::extract_call_target_address(
                ADDRESS_OF_CALL_DEFINED_STATE_IN_IDLE,
            )));
            utils::patch_call_address(
                ADDRESS_OF_CALL_DEFINED_STATE_IN_IDLE,
                hk_defined_state as usize,
            );

            Ok(())
        },
        None => Err(Error::IncompatibleSampVersion),
    }
}
