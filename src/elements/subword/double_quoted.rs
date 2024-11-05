//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::{ShellCore, Feeder};
use crate::utils::exit;
use crate::elements::word::{Word, substitution};
use crate::elements::subword::CommandSubstitution;
use super::{BracedParam, EscapedChar, SimpleSubword, Parameter, Subword, VarName};

#[derive(Debug, Clone)]
pub struct DoubleQuoted {
    text: String,
    subwords: Vec<Box<dyn Subword>>,
    split_points: Vec<usize>,
}

impl Subword for DoubleQuoted {
    fn get_text(&self) -> &str {&self.text.as_ref()}
    fn boxed_clone(&self) -> Box<dyn Subword> {Box::new(self.clone())}

    fn substitute(&mut self, core: &mut ShellCore) -> bool {
        let mut word = Word::new();
        word.subwords = self.replace_position_params(core);
        if ! substitution::eval(&mut word, core) {
            return false;
        }
        self.subwords = word.subwords;
        self.text = self.subwords.iter().map(|s| s.get_text()).collect();
        true
    }

    fn make_glob_string(&mut self) -> String {
        return self.text.replace("\\", "\\\\")
                        .replace("*", "\\*")
                        .replace("?", "\\?")
                        .replace("[", "\\[")
                        .replace("]", "\\]");
    }

    fn make_unquoted_string(&mut self) -> Option<String> {
        Some(self.subwords.iter_mut()
            .map(|s| s.make_unquoted_string())
            .filter(|s| *s != None)
            .map(|s| s.unwrap())
            .collect::<Vec<String>>()
            .concat() )
    }

    fn split(&self, _core: &mut ShellCore) -> Vec<Box<dyn Subword>>{
        if self.split_points.len() < 2 {
            return vec![];
        }

        let mut ans = vec![];

        let mut points = self.split_points.clone();
        points[self.split_points.len()-1] = self.split_points.len();

        let mut last = 0;
        for p in points {
            let mut tmp = Self::new();
            tmp.subwords = self.subwords[last..p].to_vec();
            ans.push(Box::new(tmp) as Box<dyn Subword>);
            last = p;
        }

        ans
    }
}

impl DoubleQuoted {
    pub fn new() -> DoubleQuoted {
        DoubleQuoted {
            text: String::new(),
            subwords: vec![],
            split_points: vec![],
        }
    }

    fn replace_position_params(&mut self, core: &mut ShellCore) -> Vec<Box<dyn Subword>> {
        let mut ans = vec![];

        for sw in &self.subwords {
            if sw.get_text() == "$@" {
                for pp in core.data.get_position_params() {
                    ans.push(Box::new( SimpleSubword {text: pp}) as Box<dyn Subword>);
                    self.split_points.push(ans.len());
                }
            }else{
                ans.push(sw.boxed_clone());
            }
        }
        ans
    }

    fn set_simple_subword(feeder: &mut Feeder, ans: &mut Self, len: usize) -> bool {
        if len == 0 {
            return false;
        }

        let txt = feeder.consume(len);
        ans.text += &txt;
        ans.subwords.push( Box::new(SimpleSubword{ text: txt }) );
        true
    }

    fn eat_braced_param(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> bool {
        if let Some(a) = BracedParam::parse(feeder, core){
            ans.text += a.get_text();
            ans.subwords.push(Box::new(a));
            true
        }else{
            false
        }
    }

    fn eat_command_substitution(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> bool {
        if let Some(a) = CommandSubstitution::parse(feeder, core){
            ans.text += a.get_text();
            ans.subwords.push(Box::new(a));
            true
        }else{
            false
        }
    }

    fn eat_special_or_positional_param(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> bool {
        if let Some(a) = Parameter::parse(feeder, core){
            ans.text += a.get_text();
            ans.subwords.push(Box::new(a));
            true
        }else{
            false
        }
    }

    fn eat_doller(feeder: &mut Feeder, ans: &mut Self) -> bool {
        match feeder.starts_with("$") {
            true  => Self::set_simple_subword(feeder, ans, 1),
            false => false,
        }
    }

    fn eat_escaped_char(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> bool {
        if feeder.starts_with("\\$") || feeder.starts_with("\\\\") {
            let txt = feeder.consume(2);
            ans.text += &txt;
            ans.subwords.push(Box::new(EscapedChar{ text: txt }));
            return true;
        }
        let len = feeder.scanner_escaped_char(core);
        Self::set_simple_subword(feeder, ans, len)
    }

    fn eat_name(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> bool {
        let len = feeder.scanner_name(core);
        if len == 0 {
            return false;
        }

        let txt = feeder.consume(len);
        ans.text += &txt;
        ans.subwords.push(Box::new( VarName{ text: txt}));
        true
    }

    fn eat_other(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> bool {
        let len = feeder.scanner_double_quoted_subword(core);
        Self::set_simple_subword(feeder, ans, len)
    }

    pub fn parse(feeder: &mut Feeder, core: &mut ShellCore) -> Option<DoubleQuoted> {
        if ! feeder.starts_with("\"") {
            return None;
        }
        let mut ans = Self::new();
        ans.text = feeder.consume(1);

        loop {
            while Self::eat_braced_param(feeder, &mut ans, core)
               || Self::eat_command_substitution(feeder, &mut ans, core)
               || Self::eat_special_or_positional_param(feeder, &mut ans, core)
               || Self::eat_doller(feeder, &mut ans)
               || Self::eat_escaped_char(feeder, &mut ans, core)
               || Self::eat_name(feeder, &mut ans, core)
               || Self::eat_other(feeder, &mut ans, core) {}

            if feeder.starts_with("\"") {
                ans.text += &feeder.consume(1);
//                eprintln!("{:?}", &ans);
                return Some(ans);
            }else if feeder.len() > 0 {
                exit::internal("unknown chars in double quoted word");
            }else if ! feeder.feed_additional_line(core) {
                return None;
            }
        }
    }
}
