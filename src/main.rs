extern crate getopts;
extern crate ncurses;
extern crate nix;

mod common;
mod logo;
mod c51;
mod d51;

use getopts::Options;
use std::env;
use std::{thread, time};
use ncurses::*;
use nix::sys::signal;
use nix::sys::signal::{sigaction, SigAction, SigHandler, SaFlags, SigSet};
use common::*;
use logo::Logo;
use c51::C51;
use d51::D51;

pub enum SLType {
    Logo,
    C51,
    D51
}

pub fn my_mvaddstr(y: i32, mut x: i32, str: &str) -> bool {
    let mut chars = str.chars();
    while x < 0 {
        chars.next();
        x += 1;
    }
    while let Some(c) = chars.next() {
        if mvaddch(y, x, c as chtype) == ERR { return false }
        x += 1;
    }

    true
}

pub struct Config {
    pub accident: bool,
    pub fly: bool,
    pub smoke: bool,
    pub smoke_state: smoke::SmokeState
}

pub trait Train {
    fn update(&mut self, x: i32) -> bool;
    fn get_smoke_state(&mut self) -> &mut smoke::SmokeState;

    fn run(&mut self) {
        initscr();
        
        let action = SigAction::new(SigHandler::SigIgn, SaFlags::empty(), SigSet::empty());
        unsafe { sigaction(signal::SIGINT, &action) }.unwrap();

        noecho();
        curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
        nodelay(stdscr(), true);
        leaveok(stdscr(), true);
        scrollok(stdscr(), false);

        let mut x = COLS() - 1;
        loop {
            if !self.update(x) {
                break;
            }

            getch();
            refresh();
            thread::sleep(time::Duration::from_millis(40));
            x -= 1;
        }

        mvcur(0, COLS() - 1, LINES() - 1, 0);
        endwin();
    }


    fn add_man(&self, y: i32, x: i32) {
        for i in 0..2 {
            let man_x = ((SL_LENGTH + x) / 12 % 2) as usize;
            my_mvaddstr(y + i, x, MAN[man_x][i as usize]);
        }
    }

    fn add_smoke(&mut self, y: i32, x: i32) {
        use smoke::*;
        let mut state = self.get_smoke_state();
        let sum: usize = state.sum;
        let s = &mut state.s;

        if x % 4 == 0 {
            for i in 0..sum {
                let pattern = s[i].ptrn as usize;
                my_mvaddstr(s[i].y, s[i].x, ERASER[pattern]);
                s[i].y -= DY[pattern];
                s[i].x += DX[pattern];
                let pattern = if pattern < SMOKEPTNS - 1 {
                    s[i].ptrn += 1;
                    s[i].ptrn as usize
                } else { pattern };

                my_mvaddstr(s[i].y, s[i].x, SMOKE[(s[i].kind) as usize][pattern]);
            }
            my_mvaddstr(y, x, SMOKE[sum % 2][0]);
            s[sum].y = y;
            s[sum].x = x;
            s[sum].ptrn = 0;
            s[sum].kind = (sum % 2) as i32;
            state.sum = sum + 1;
        }
    }
}

fn print_usage(program: &str, opts: Options) {
    println!("{}", opts.usage(&format!("Usage:\n {} [options]", program)));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("l", "logo", "select logo");
    opts.optflag("c", "c51", "select C51");
    opts.optflag("F", "fly", "enable fly mode");
    opts.optflag("a", "accident", "enable accident mode");
    opts.optflag("s", "smoke", "(not yet implemented) enable smoke mode");
    opts.optflag("", "help", "show this usage message.");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(_) => {
            print_usage(&program, opts);
            return;
        }
    };
    if matches.opt_present("help") {
        print_usage(&program, opts);
        return;
    }
    let sl_type =
        if matches.opt_present("logo") {
            SLType::Logo
        } else if matches.opt_present("c51") {
            SLType::C51
        } else {
            SLType::D51
        };

    let conf = Config {
        accident: matches.opt_present("accident"),
        fly: matches.opt_present("fly"),
        smoke: matches.opt_present("smoke"),
        smoke_state: smoke::SmokeState::new()
    };
    match sl_type {
        SLType::Logo => Logo::new(conf).run(),
        SLType::C51 => C51::new(conf).run(),
        SLType::D51 => D51::new(conf).run()
    };
}
