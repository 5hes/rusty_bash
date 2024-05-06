//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::ShellCore;
use crate::core::builtins::completion;
use crate::feeder::terminal::Terminal;
use std::path::Path;
use unicode_width::UnicodeWidthStr;

fn common_length(chars: &Vec<char>, s: &String) -> usize {
    let max_len = chars.len();
    for (i, c) in s.chars().enumerate() {
        if i >= max_len || chars[i] != c {
            return i;
        }
    }
    max_len
}

fn common_string(paths: &Vec<String>) -> String {
    if paths.len() == 0 {
        return "".to_string();
    }

    let ref_chars: Vec<char> = paths[0].chars().collect();
    let mut common_len = ref_chars.len();

    for path in &paths[1..] {
        let len = common_length(&ref_chars, &path);
        common_len = std::cmp::min(common_len, len);
    }

    ref_chars[..common_len].iter().collect()
}

fn is_dir(s: &str, core: &mut ShellCore) -> bool {
    let tilde_prefix = "~/".to_string();
    let tilde_path = core.data.get_param_ref("HOME").to_string() + "/";

    Path::new(&s.replace(&tilde_prefix, &tilde_path)).is_dir()
}

impl Terminal {
    pub fn completion(&mut self, core: &mut ShellCore, double_tab: bool) {
        self.set_completion_info(core);

        if ! self.set_default_compreply(core) {
            self.cloop();
            return;
        }

        match double_tab {
            true  => self.show_list(&core.data.arrays["COMPREPLY"]),
            false => self.try_completion(core),
        }
    }

    pub fn set_default_compreply(&mut self, core: &mut ShellCore) -> bool {
        let pos = core.data.get_param_ref("COMP_CWORD").to_string();
        let last = core.data.get_array("COMP_WORDS", &pos);

        let (tilde_prefix, tilde_path, last_tilde_expanded) = Self::set_tilde_transform(&last, core);


        let mut args = vec!["".to_string(), "".to_string(), last_tilde_expanded.to_string()];
        let list = match pos == "0" {
            true  => completion::compgen_c(core, &mut args),
            false => completion::compgen_f(core, &mut args),
        };

        if list.len() == 0 {
            return false;
        }

        let tmp = list.iter().map(|p| p.replacen(&tilde_path, &tilde_prefix, 1)).collect();
        core.data.set_array("COMPREPLY", &tmp);
        true
    }

    pub fn try_completion(&mut self, core: &mut ShellCore) {
        let pos = core.data.get_param_ref("COMP_CWORD").to_string();
        let target = core.data.get_array("COMP_WORDS", &pos);

        if core.data.arrays["COMPREPLY"].len() == 1 {
            let output = core.data.arrays["COMPREPLY"][0].clone();
            let tail = match is_dir(&output, core) {
                true  => "/",
                false => " ",
            };
            self.replace_input(&(output + tail));
            return;
        }

        let common = common_string(&core.data.arrays["COMPREPLY"]);
        if common.len() != target.len() {
            self.replace_input(&common);
            return;
        }
        self.cloop();
    }

    fn show_list(&mut self, list: &Vec<String>) {
        eprintln!("\r");

        let widths: Vec<usize> = list.iter()
                                     .map(|p| UnicodeWidthStr::width(p.as_str()))
                                     .collect();
        let max_entry_width = widths.iter().max().unwrap_or(&1000) + 1;

        let col_num = Terminal::size().0 / max_entry_width;
        if col_num == 0 {
            list.iter().for_each(|p| print!("{}\r\n", &p));
            self.rewrite(true);
            return;
        }

        let row_num = (list.len()-1) / col_num + 1;

        for row in 0..row_num {
            for col in 0..col_num {
                let i = col*row_num + row;
                if i >= list.len() {
                    continue;
                }

                let space_num = max_entry_width - widths[i];
                let s = String::from_utf8(vec![b' '; space_num]).unwrap();
                print!("{}{}", list[i], &s);
            }
            print!("\r\n");
        }
        self.rewrite(true);
    }

    fn replace_input(&mut self, to: &String) {
        let min = self.prompt.chars().count();
        while min < self.head && self.head > 0 && self.chars[self.head-1] != ' ' {
            self.backspace();
        }
        while self.head < self.chars.len() && self.chars[self.head] != ' ' {
            self.delete();
        }

        for c in to.chars() {
            self.insert(c);
        }
    }

    fn set_tilde_transform(last: &str, core: &mut ShellCore) -> (String, String, String) {
        let tilde_prefix;
        let tilde_path;
        let last_tilde_expanded;

        if last.starts_with("~/") {
            tilde_prefix = "~/".to_string();
            tilde_path = core.data.get_param_ref("HOME").to_string() + "/";
            last_tilde_expanded = last.replacen(&tilde_prefix, &tilde_path, 1);
        }else{
            tilde_prefix = String::new();
            tilde_path = String::new();
            last_tilde_expanded = last.to_string();
        }

        (tilde_prefix, tilde_path, last_tilde_expanded)
    }

    fn set_completion_info(&mut self, core: &mut ShellCore){
        let pcc = self.prompt.chars().count();
        let s = self.get_string(pcc);
        let mut ws = s.split(" ").map(|e| e.to_string()).collect::<Vec<String>>();
        ws.retain(|e| e != "");
        core.data.set_array("COMP_WORDS", &ws);

        let s: String = self.chars[pcc..self.head].iter().collect();
        let mut ws = s.split(" ").map(|e| e.to_string()).collect::<Vec<String>>();
        ws.retain(|e| e != "");
        let mut num = ws.len();

        match s.chars().last() {
            Some(' ') => {},
            Some(_) => num -= 1,
            _ => {},
        }
        core.data.set_param("COMP_CWORD", &num.to_string());
    }
}
