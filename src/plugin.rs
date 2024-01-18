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

use egui::{Color32, epaint::Shadow, FontData, FontDefinitions, FontFamily, FontId, FontTweak, Key, Label, RichText, Rounding, Sense, TextStyle};
use egui_d3d9::EguiDx9;
use vmt_hook::VTableHook;
use windows::{
    core::{HRESULT, w},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::{
            Direct3D9::{D3DPRESENT_PARAMETERS, IDirect3DDevice9},
            Gdi::RGNDATA,
        },
        System::LibraryLoader::GetModuleHandleW,
    },
};
use windows::Win32::UI::WindowsAndMessaging::{CallWindowProcA, GWLP_WNDPROC, SetWindowLongPtrA, WM_LBUTTONDOWN, WNDPROC};

use crate::{gta, samp, sampfuncs, utils};
use crate::sampfuncs::{CmdOwner, CommandType};

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

type CommandMap = HashMap<String, String>;
type ModuleMap = HashMap<String, CommandMap>;

struct Category {
    is_visible: bool,
    name: String,
    modules: ModuleMap,
}

struct Categories {
    samp: Category,
    sf: Category,
    cleo: Category,
}

enum CategoryKey {
    Samp,
    SfPlugin,
    Cleo,
}

impl Categories {
    pub fn is_empty(&self) -> bool {
        self.samp.modules.is_empty() && self.sf.modules.is_empty() && self.cleo.modules.is_empty()
    }
}

impl std::ops::Index<&CategoryKey> for Categories {
    type Output = Category;

    fn index(&self, index: &CategoryKey) -> &Self::Output {
        match index {
            CategoryKey::Samp => &self.samp,
            CategoryKey::SfPlugin => &self.sf,
            CategoryKey::Cleo => &self.cleo,
        }
    }
}

pub struct Plugin {
    d3d9_hook: Option<VTableHook<IDirect3DDevice9>>,
    gui: Option<EguiDx9<()>>,
    commands: Categories,
    commands_order: [CategoryKey; 3],
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
                samp: Category { is_visible: false, name: "SA-MP".to_string(), modules: ModuleMap::new() },
                sf: Category { is_visible: false, name: "SF".to_string(), modules: ModuleMap::new() },
                cleo: Category { is_visible: false, name: "CLEO".to_string(), modules: ModuleMap::new() },
            },
            commands_order: [CategoryKey::Samp, CategoryKey::SfPlugin, CategoryKey::Cleo],
            original_wnd_proc: None,
            original_reset: None,
            original_present: None,
            samp_base_address,
            samp_version,
        }
    }

    pub fn post_initialize(&mut self) {
        unsafe {
            self.install_wnd_proc();
            self.install_d3d9_hooks();
            self.init_egui();
        }
    }

    fn cmd_with_prefix(command: &str) -> String {
        "/".to_string() + command
    }

    pub fn parse_commands(&mut self) {
        // Todo: Prefer placing hooks on command registration and removal rather than parsing them once.

        let samp_cmds: HashMap<String, Vec<String>> = self.get_samp_commands_grouped_by_module();
        let samp_modules = samp_cmds
            .into_iter()
            .map(|(module, cmds)| (module, cmds.into_iter().map(|cmd| (Self::cmd_with_prefix(&cmd), String::default())).collect()))
            .collect();
        let samp = &mut self.commands.samp;
        samp.modules = samp_modules;
        samp.is_visible = true;

        if let Some(sf_cmds) = self.get_sampfuncs_commands_grouped() {
            let mut sf_modules = ModuleMap::new();
            let mut cleo_modules = ModuleMap::new();

            for (module, v) in sf_cmds {
                match v.0 {
                    CommandType::PLUGIN => { sf_modules.entry(module).or_insert(v.1.into_iter().map(|cmd| (Self::cmd_with_prefix(&cmd), String::default())).collect()); }
                    CommandType::SCRIPT => { cleo_modules.entry(module).or_insert(v.1.into_iter().map(|cmd| (Self::cmd_with_prefix(&cmd), String::default())).collect()); }
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
        let device = gta::get_d3d9_device();
        let hook = VTableHook::with_count(device, 119);

        self.original_reset = Some(std::mem::transmute(hook.get_original_method(16)));
        self.original_present = Some(std::mem::transmute(hook.get_original_method(17)));

        hook.hook_method(16, Self::hk_reset as usize);
        hook.hook_method(17, Self::hk_present as usize);

        self.d3d9_hook = Some(hook);
    }

    fn init_egui(&mut self) {
        // *mut IDirect3DDevice9 is the same as IDirect3DDevice9.
        let device = unsafe { std::mem::transmute(gta::get_d3d9_device()) };
        let window = gta::get_window_handle();

        let gui = EguiDx9::init(&device, window, Self::render_ui, (), true);

        let ctx = gui.ctx();
        Self::setup_custom_fonts(ctx);
        Self::configure_text_styles(ctx);
        Self::configure_visuals(ctx);

        self.gui = Some(gui);
    }

    unsafe extern "stdcall" fn hk_reset(
        device: IDirect3DDevice9,
        presentation_parameters: *const D3DPRESENT_PARAMETERS,
    ) -> HRESULT {
        let plugin = PLUGIN.as_mut().unwrap_unchecked();
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
        let plugin = PLUGIN.as_mut().unwrap_unchecked();
        let gui = plugin.gui.as_mut().unwrap_unchecked();
        gui.present(&device);

        let original_present = plugin.original_present.unwrap_unchecked();
        original_present(device, source_rect, dest_rect, dest_window_override, dirty_region)
    }

    unsafe extern "stdcall" fn hk_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let plugin = PLUGIN.as_mut().unwrap_unchecked();
        let gui = plugin.gui.as_mut().unwrap_unchecked();
        gui.wnd_proc(msg, wparam, lparam);

        if msg == WM_LBUTTONDOWN && gui.ctx().wants_pointer_input() {
            // To prevent the chat from closing when clicking on our interface.
            LRESULT(1)
        } else {
            CallWindowProcA(plugin.original_wnd_proc.unwrap_unchecked(), hwnd, msg, wparam, lparam)
        }
    }

    fn add_font(fonts: &mut FontDefinitions, name: &str, font: &'static [u8]) {
        let name = name.to_string();
        let tweak = FontTweak::default();
        fonts.font_data.insert(
            name.clone(),
            FontData::from_static(font).tweak(tweak),
        );
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, name.clone());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .push(name);
    }

    fn setup_custom_fonts(ctx: &egui::Context) {
        let mut fonts = FontDefinitions::default();
        Self::add_font(&mut fonts, "Segoe UI Bold", include_bytes!("C:\\Windows\\Fonts\\segoeuib.ttf"));
        ctx.set_fonts(fonts);
    }

    fn configure_text_styles(ctx: &egui::Context) {
        use FontFamily::{Monospace, Proportional};

        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (TextStyle::Heading, FontId::new(24.0, Proportional)),
            (TextStyle::Body, FontId::new(16.5, Proportional)),
            (TextStyle::Monospace, FontId::new(16.0, Monospace)),
            (TextStyle::Button, FontId::new(16.5, Proportional)),
            (TextStyle::Small, FontId::new(8.0, Proportional)),
        ].into();
        ctx.set_style(style);
    }

    fn configure_visuals(ctx: &egui::Context) {
        let mut visuals = ctx.style().visuals.clone();
        visuals.window_shadow = Shadow::NONE;
        visuals.window_fill = Color32::from_rgba_premultiplied(20, 20, 20, 200);
        visuals.window_rounding = Rounding::same(10.);
        ctx.set_visuals(visuals);
    }

    fn render_ui(ctx: &egui::Context, _: &mut ()) {
        if gta::is_gta_menu_active() {
            return;
        }

        // SA-MP keys.
        // Todo: It might be better to read the key from memory, in case there is a plugin to change the keys.
        if ctx.input(|i| i.key_down(Key::F5) || i.key_down(Key::F10)) {
            return;
        }

        let input = match samp::Input::get() {
            Some(v) => v,
            None => return,
        };

        // Draw only if chat input is open.
        if !input.enabled.as_bool() {
            return;
        }

        let chat_input = input.edit_box().get_text();

        let plugin = unsafe { PLUGIN.as_ref().unwrap_unchecked() };
        let chat_contains_cmd = chat_input.starts_with('/');
        // Don't draw empty list.
        if (input.total_recall == 0 && !chat_contains_cmd) || (chat_contains_cmd && plugin.commands.is_empty()) {
            return;
        }

        let pos = input.edit_box().position;
        let pos = [pos[0] as f32, (pos[1] + input.edit_box().height + 5) as f32];

        egui::containers::Window::new("Command Helper")
            .fixed_pos(pos)
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if chat_contains_cmd {
                    egui::Grid::new("cmds")
                        .min_col_width(150.0)
                        .show(ui, |ui| {
                            for category_key in &plugin.commands_order {
                                let category = &plugin.commands[category_key];
                                if category.is_visible {
                                    ui.vertical_centered(|ui| {
                                        ui.strong(&category.name);
                                    });
                                }
                            }
                            ui.end_row();

                            for category_key in &plugin.commands_order {
                                let category = &plugin.commands[category_key];
                                if category.is_visible {
                                    ui.vertical(|ui| {
                                        for (name, commands) in &category.modules {
                                            egui::CollapsingHeader::new(name).default_open(true).show(ui, |ui| {
                                                for (cmd, description) in commands {
                                                    let text = if chat_input.is_empty() || cmd.starts_with(&chat_input) {
                                                        RichText::new(cmd)
                                                    } else {
                                                        RichText::new(cmd).weak()
                                                    };

                                                    let label = ui.add(Label::new(text).sense(Sense::click()));

                                                    if label.clicked() {
                                                        input.edit_box().set_text(cmd.as_str());
                                                    }

                                                    if !description.is_empty() {
                                                        label.on_hover_text(description);
                                                    }
                                                }
                                            });
                                        }
                                    });
                                }
                            }
                            ui.end_row();
                        });
                } else {
                    ui.vertical_centered(|ui| {
                        ui.strong("Recalls");
                    });

                    ui.indent(ui.id(), |ui| {
                        for i in 0..input.total_recall as usize {
                            if let Ok(recall) = CStr::from_bytes_until_nul(&input.recall_buffer[i]) {
                                if let Ok(text) = recall.to_str() {
                                    let text = if input.current_recall == -1 || i == input.current_recall as usize {
                                        RichText::new(text)
                                    } else {
                                        RichText::new(text).weak()
                                    };

                                    let label = ui.add(Label::new(text).sense(Sense::click()));

                                    if label.clicked() {
                                        input.current_recall = i as i32;
                                        input.edit_box().set_text_raw(recall.as_ptr());
                                    }
                                }
                            }
                        }
                    });
                }

                ui.separator();
                ui.vertical_centered(|ui| {
                    ui.strong("Copyright © Rinat Namazov")
                        .on_hover_ui(|ui| {
                            ui.label(concat!("SA-MP Command Helper v", env!("CARGO_PKG_VERSION")));
                            ui.label("https://rinwares.com");
                        });
                });
            });
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

    fn get_sampfuncs_commands_grouped(&self) -> Option<HashMap<String, (CommandType, Vec<String>)>> {
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
            if let Some(plugin) = PLUGIN.as_mut() {
                samp::initialize(plugin.samp_base_address, plugin.samp_version);
                let _ = sampfuncs::initialize();
                plugin.post_initialize();
            }
            STATE = InitState::Initialized;
        }
        InitState::Initialized => {
            static mut TIME: OnceCell<SystemTime> = OnceCell::new();
            let time = TIME.get_or_init(|| SystemTime::now());

            if time.elapsed().unwrap() > Duration::from_secs(3) {
                if let Some(plugin) = PLUGIN.as_mut() {
                    plugin.parse_commands();
                }
                STATE = InitState::Nothing;
            }
        }
        InitState::Nothing => {}
    }
}

unsafe extern "C" fn hk_defined_state() {
    initialize_plugin();
    FUNC_GTA_DEFINED_STATE.unwrap()();
}

pub fn initialize() {
    const ADDRESS_OF_CALL_DEFINED_STATE_IN_IDLE: usize = 0x53EA8E;

    let current_byte = unsafe { std::ptr::read(ADDRESS_OF_CALL_DEFINED_STATE_IN_IDLE as *const u8) };
    if current_byte != 0xE8 /* call opcode */ {
        panic!("samp-cmd-helper: the plugin has detected that it's maybe loaded into the wrong game.");
    }

    let samp_base_address = unsafe { GetModuleHandleW(w!("samp.dll")).unwrap() };
    if samp_base_address.is_invalid() {
        panic!("samp-cmd-helper: the plugin didn't detect the loaded 'samp.dll'.");
    }
    let samp_base_address = samp_base_address.0 as usize;

    match samp::get_version(samp_base_address) {
        Some(samp_version) => unsafe {
            PLUGIN = Some(Plugin::new(samp_base_address, samp_version));

            FUNC_GTA_DEFINED_STATE = Some(std::mem::transmute(utils::extract_call_target_address(ADDRESS_OF_CALL_DEFINED_STATE_IN_IDLE)));
            utils::patch_call_address(ADDRESS_OF_CALL_DEFINED_STATE_IN_IDLE, hk_defined_state as usize);
        }
        None => panic!("samp-cmd-helper: the plugin didn't detect a compatible version of 'samp.dll'."),
    }
}
