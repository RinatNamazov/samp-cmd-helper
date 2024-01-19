/*****************************************************************************
 *
 *  PROJECT:        samp-cmd-helper
 *  LICENSE:        See LICENSE in the top level directory
 *  FILE:           cmd_storage.rs
 *  DESCRIPTION:    Command storage structures
 *  COPYRIGHT:      (c) 2024 RINWARES <rinwares.com>
 *  AUTHOR:         Rinat Namazov <rinat.namazov@rinwares.com>
 *
 *****************************************************************************/

use std::collections::HashMap;

pub const CMD_PREFIX: &str = "/";

pub type CommandMap = HashMap<String, String>;
pub type ModuleMap = HashMap<String, CommandMap>;

pub struct Category {
    pub is_visible: bool,
    pub name: String,
    pub modules: ModuleMap,
}

impl Category {
    pub fn new(name: String) -> Self {
        Self {
            is_visible: false,
            name,
            modules: ModuleMap::new(),
        }
    }
}

pub enum CategoryKey {
    Samp,
    SfPlugin,
    Cleo,
    Lua,
}

pub struct Categories {
    pub order: [CategoryKey; 4],
    pub samp: Category,
    pub sf: Category,
    pub cleo: Category,
    pub lua: Category,
}

impl Categories {
    pub fn is_empty(&self) -> bool {
        self.samp.modules.is_empty()
            && self.sf.modules.is_empty()
            && self.cleo.modules.is_empty()
            && self.lua.modules.is_empty()
    }

    pub fn iter(&self) -> CategoriesIterator {
        CategoriesIterator {
            categories: self,
            current_index: 0,
        }
    }
}

impl std::ops::Index<&CategoryKey> for Categories {
    type Output = Category;

    fn index(&self, index: &CategoryKey) -> &Self::Output {
        match index {
            CategoryKey::Samp => &self.samp,
            CategoryKey::SfPlugin => &self.sf,
            CategoryKey::Cleo => &self.cleo,
            CategoryKey::Lua => &self.lua,
        }
    }
}

pub struct CategoriesIterator<'a> {
    categories: &'a Categories,
    current_index: usize,
}

impl<'a> Iterator for CategoriesIterator<'a> {
    type Item = &'a Category;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index < self.categories.order.len() {
            let key = &self.categories.order[self.current_index];
            self.current_index += 1;
            Some(&self.categories[key])
        } else {
            None
        }
    }
}

pub fn cmd_with_prefix(command: &str) -> String {
    let mut str = String::with_capacity(CMD_PREFIX.len() + command.len());
    str.push_str(CMD_PREFIX);
    str.push_str(command);
    str
}
