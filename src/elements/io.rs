//SPDX-FileCopyrightText: 2022 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

pub mod pipe;
pub mod redirect;

use std::os::unix::prelude::RawFd;
use nix::{fcntl, unistd};

fn close(fd: RawFd, err_str: &str){
    if fd >= 0 {
        unistd::close(fd).expect(err_str);
    }
}

fn replace(from: RawFd, to: RawFd) {
    if from < 0 || to < 0 {
        return;
    }
    unistd::dup2(from, to).expect("Can't copy file descriptors");
    close(from, &("Can't close fd: ".to_owned() + &from.to_string()))
}

fn share(from: RawFd, to: RawFd) -> bool {
    if from < 0 || to < 0 {
        return false;
    }

    if let Ok(_) = unistd::dup2(from, to) {
        true
    }else{
        false
    }
}

fn backup(from: RawFd) -> RawFd {
    fcntl::fcntl(from, fcntl::F_DUPFD_CLOEXEC(10))
           .expect("Can't allocate fd for backup")
}
