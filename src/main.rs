extern crate crossterm;
extern crate getopts;

use std::{env, io};
use std::io::Write;
use std::thread;
use std::time::{Duration, Instant};

use crossterm::{Crossterm, input, InputEvent, KeyEvent, RawScreen, TerminalCursor};
use getopts::Options;

use c51::C51;
use common::*;
use d51::D51;
use logo::Logo;

mod common;
mod logo;
mod c51;
mod d51;

pub enum SLType {
    Logo,
    C51,
    D51,
}

pub struct Terminal {
    term: crossterm::Terminal,
    pub cursor: TerminalCursor,
    pub cols: i32,
    pub lines: i32,
}

impl Terminal {
    pub fn new() -> Terminal {
        let crossterm = Crossterm::new();
        let term = crossterm.terminal();
        let (cols, lines) = term.terminal_size();
        Terminal {
            term,
            cursor: crossterm.cursor(),
            cols: From::from(cols),
            lines: From::from(lines),
        }
    }

    pub fn init(&self) -> io::Result<()> {
        self.clear_all()?;
        self.cursor.hide()?;
        Ok(())
    }

    fn finish(&self) -> io::Result<()> {
        self.clear_all()?;
        let (_, lines) = self.term.terminal_size();
        self.cursor.goto(0, lines)?;
        self.cursor.show()?;
        Ok(())
    }

    pub fn clear_all(&self) -> io::Result<()> {
        self.term.clear(crossterm::ClearType::All)?;
        Ok(())
    }

    pub fn mvaddstr(&self, y: i32, mut x: i32, str: &str) -> bool {
        let mut chars = str.chars();
        while x < 0 {
            chars.next();
            x += 1;
        }
        for c in chars {
            if self.cursor.goto(x as u16, y as u16).is_err() {
                return false;
            }
            if self.term.write(c).is_err() {
                return false;
            }

            x += 1;
        }

        true
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.finish().expect("drop Terminal");
    }
}

pub struct Config {
    pub accident: bool,
    pub fly: bool,
    pub smoke: bool,
    pub smoke_state: smoke::SmokeState,
    pub interruptable: bool,
}

pub trait Train {
    fn update(&mut self, terminal: &Terminal, x: i32) -> bool;
    fn get_smoke_state(&mut self) -> &mut smoke::SmokeState;
    fn config(&self) -> &Config;

    fn run(&mut self) -> io::Result<()> {
        let terminal = Terminal::new();
        terminal.init()?;
        let mut stdin = input().read_async();
        let _screen = RawScreen::into_raw_mode()?;
        let mut interrupted = false;
        let frame_duration = Duration::from_millis(40);
        let mut next_frame_time = Instant::now() + frame_duration;

        let mut x = terminal.cols;
        while !interrupted {
            if !self.update(&terminal, x) {
                break;
            }

            loop {
                match stdin.next() {
                    Some(InputEvent::Keyboard(KeyEvent::Ctrl(key))) if key == 'c' => {
                        if self.config().interruptable { interrupted = true }
                    }
                    Some(_) => (),
                    None => break,
                }
            }

            io::stdout().flush()?;
            // Instant.checked_duration_since() is unstable feature
            if let Some(duration) = checked_duration_since(next_frame_time, Instant::now()) {
                thread::sleep(duration);
            }
            next_frame_time += frame_duration;
            x -= 1;
        }

        Ok(())
    }


    fn add_man(&self, terminal: &Terminal, y: i32, x: i32) {
        for i in 0..2 {
            let man_x = ((SL_LENGTH + x) / 12 % 2) as usize;
            terminal.mvaddstr(y + i, x, MAN[man_x][i as usize]);
        }
    }

    fn add_smoke(&mut self, terminal: &Terminal, y: i32, x: i32) {
        use smoke::*;
        let state = self.get_smoke_state();
        let sum: usize = state.sum;
        let s = &mut state.s;

        if x % 4 == 0 {
            for i in 0..sum {
                let pattern = s[i].ptrn as usize;
                terminal.mvaddstr(s[i].y, s[i].x, ERASER[pattern]);
                s[i].y -= DY[pattern];
                s[i].x += DX[pattern];
                let pattern = if pattern < SMOKEPTNS - 1 {
                    s[i].ptrn += 1;
                    s[i].ptrn as usize
                } else { pattern };

                terminal.mvaddstr(s[i].y, s[i].x, SMOKE[(s[i].kind) as usize][pattern]);
            }
            terminal.mvaddstr(y, x, SMOKE[sum % 2][0]);
            s[sum].y = y;
            s[sum].x = x;
            s[sum].ptrn = 0;
            s[sum].kind = (sum % 2) as i32;
            state.sum = sum + 1;
        }
    }
}

fn checked_duration_since(s: Instant, earlier: Instant) -> Option<Duration> {
    if s > earlier {
        Some(s - earlier)
    } else {
        None
    }
}

fn print_usage(program: &str, opts: &Options) {
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
    opts.optflag("s", "no-smoke", "disable smoke mode");
    opts.optflag("i", "interrupt", "enable Ctrl-C interrupt");
    opts.optflag("", "help", "show this usage message.");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(_) => {
            print_usage(&program, &opts);
            return;
        }
    };
    if matches.opt_present("help") {
        print_usage(&program, &opts);
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
        smoke: !matches.opt_present("no-smoke"),
        smoke_state: smoke::SmokeState::new(),
        interruptable: matches.opt_present("interrupt"),
    };
    match sl_type {
        SLType::Logo => Logo::new(conf).run(),
        SLType::C51 => C51::new(conf).run(),
        SLType::D51 => D51::new(conf).run()
    }.expect("Train run");
}
