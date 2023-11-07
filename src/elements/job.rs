//SPDX-FileCopyrightText: 2022 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use super::pipeline::Pipeline;
use crate::{Feeder, ShellCore};
use crate::core::jobtable::JobEntry;
use nix::unistd;
use nix::unistd::{Pid, ForkResult};

#[derive(Debug)]
pub struct Job {
    pub pipelines: Vec<Pipeline>,
    pub pipeline_ends: Vec<String>,
    pub text: String,
}

impl Job {
    pub fn exec(&mut self, core: &mut ShellCore, bg: bool) {
        let pgid = if core.is_subshell { //17〜21行目を追加
            unistd::getpgrp() //自身のPGID
        }else{
            Pid::from_raw(0)
        };

        core.jobtable_status_check();
        if bg {
            self.exec_bg(core, pgid);
        }else{
            self.exec_fg(core, pgid);
        }
    }

    fn exec_fg(&mut self, core: &mut ShellCore, pgid: Pid) {
        let mut do_next = true;
        for (pipeline, end) in self.pipelines.iter_mut()
                          .zip(self.pipeline_ends.iter()) {
            if do_next {
                let pids = pipeline.exec(core, pgid);
                core.wait_pipeline(pids);
            }
            do_next = (&core.vars["?"] == "0") == (end == "&&");
        }
    }

    fn exec_bg(&mut self, core: &mut ShellCore, pgid: Pid) {
        let backup = core.tty_fd;
        core.tty_fd = -1;

        if self.pipelines.len() == 1 {
            let pids = self.pipelines[0].exec(core, pgid);
            core.job_table.push(JobEntry::new(pids));
        }else{
            let pid = self.exec_fork_bg(core, pgid);
            core.job_table.push(JobEntry::new(vec![pid]));
        }

        core.tty_fd = backup;
    }

    fn exec_fork_bg(&mut self, core: &mut ShellCore, pgid: Pid) -> Option<Pid> {
        match unsafe{unistd::fork()} {
            Ok(ForkResult::Child) => {
                core.initialize_as_subshell(Pid::from_raw(0), pgid);
                self.exec(core, false);
                core.exit()
            },
            Ok(ForkResult::Parent { child } ) => {
                core.set_pgid(child, pgid);
                Some(child) 
            },
            Err(err) => panic!("sush(fatal): Failed to fork. {}", err),
        }
    }

    fn new() -> Job {
        Job {
            text: String::new(),
            pipeline_ends: vec![],
            pipelines: vec![],
        }
    }

    fn eat_blank_line(feeder: &mut Feeder, ans: &mut Job, core: &mut ShellCore) -> bool {
        let num = feeder.scanner_blank(core);
        ans.text += &feeder.consume(num);
        let com_num = feeder.scanner_comment();
        ans.text += &feeder.consume(com_num);
        if feeder.starts_with("\n") {
            ans.text += &feeder.consume(1);
            true
        }else{
            false
        }
    }

    fn eat_pipeline(feeder: &mut Feeder, ans: &mut Job, core: &mut ShellCore) -> bool {
        match Pipeline::parse(feeder, core){
            Some(pipeline) => {
                ans.text += &pipeline.text.clone();
                ans.pipelines.push(pipeline);
                true
            },
            None => false,
        }
    }

    fn eat_and_or(feeder: &mut Feeder, ans: &mut Job, core: &mut ShellCore) -> bool {
        let num = feeder.scanner_and_or(core);
        let end = feeder.consume(num);
        ans.pipeline_ends.push(end.clone());
        ans.text += &end;
        num != 0 //記号なしの場合にfalseが返る
    }

    pub fn parse(feeder: &mut Feeder, core: &mut ShellCore) -> Option<Job> {
        let mut ans = Self::new();
        while Self::eat_blank_line(feeder, &mut ans, core) {} 
        if ! Self::eat_pipeline(feeder, &mut ans, core) {
            return None;
        }

        while Self::eat_and_or(feeder, &mut ans, core) { 
            loop {
                while Self::eat_blank_line(feeder, &mut ans, core) {} 
                if Self::eat_pipeline(feeder, &mut ans, core) {
                    break;  
                }
                if feeder.len() != 0 || ! feeder.feed_additional_line(core) {
                    return None;
                }
            }
        }
    
        if ans.pipelines.len() > 0 {
//            dbg!("{:?}", &ans); // デバッグ用にansの内容を出力
            Some(ans)
        }else{
            None
        }
    }
}
