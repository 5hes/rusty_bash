//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::{ShellCore, Feeder};
use crate::elements::subword::Subword;

#[derive(Debug, Clone)]
pub struct SingleQuoted {
    text: String,
}

impl Subword for SingleQuoted {
    fn get_text(&self) -> &str {&self.text.as_ref()}
    fn boxed_clone(&self) -> Box<dyn Subword> {Box::new(self.clone())}

    fn make_glob_string(&mut self) -> String {
        self.text.replace("\\", "\\\\")
            .replace("*", "\\*")
            .replace("?", "\\?")
            .replace("[", "\\[")
            .replace("]", "\\]")
    }

    fn is_quoted(&self) -> bool {true}
}

impl SingleQuoted {
    pub fn parse(feeder: &mut Feeder, core: &mut ShellCore) -> Option<Self> {
        match feeder.scanner_single_quoted_subword(core) {
            0 => None,
            n => {
                let s = feeder.consume(n);
                Some(SingleQuoted{ text: s[1..n-1].to_string() })
            },
        }
    }
}