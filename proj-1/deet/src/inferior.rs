use crate::dwarf_data::{DwarfData, Error as DwarfError};
use std::mem::size_of;
use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::process::{Child, Command};
use std::os::unix::process::CommandExt;
use nix::sys::stat::stat;
use core::num::FpCategory::Infinite;
use std::collections::HashMap;
use std::borrow::Borrow;

pub enum Status {
    /// Indicates inferior stopped. Contains the signal that stopped the process, as well as the
    /// current instruction pointer that it is stopped at.
    Stopped(signal::Signal, usize),

    /// Indicates inferior exited normally. Contains the exit status code.
    Exited(i32),

    /// Indicates the inferior exited due to a signal. Contains the signal that killed the
    /// process.
    Signaled(signal::Signal),
}

/// This function calls ptrace with PTRACE_TRACEME to enable debugging on a process. You should use
/// pre_exec with Command to call this in the child process.
fn child_traceme() -> Result<(), std::io::Error> {
    ptrace::traceme().or(Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "ptrace TRACEME failed",
    )))
}

pub struct Inferior {
    child: Child,
    is_exited: bool,
    is_brk_point: bool,
    brk_point_rip: Option<usize>
}

impl Inferior {
    /// Attempts to start a new inferior process. Returns Some(Inferior) if successful, or None if
    /// an error is encountered.
    pub fn new(target: &str, args: &Vec<String>, break_points: &Vec<usize>,
               brk_point_map: &mut HashMap<usize, u8>) -> Option<Inferior> {
        // TODO: implement me!
        let mut  cmd = Command::new(target);
        cmd.args(args);
        // Nearly the same issue like the below one:
        // https://stackoverflow.com/questions/54056268/temporary-value-is-freed-at-the-end-of-this-statement

        unsafe {
            cmd.pre_exec(child_traceme);
        }
        let child = cmd.spawn().ok()?;
        let mut the_inferior = Inferior{child, is_exited: false, is_brk_point: false, brk_point_rip: None };
        the_inferior.wait(Some(WaitPidFlag::WUNTRACED)).ok()?;

        //set the break_point
        for break_point in break_points {
            let orig_byte = the_inferior.write_byte(break_point.clone(), 0xcc)
                .ok().unwrap();
            brk_point_map.insert(break_point.clone(), orig_byte);
        }

        Some(the_inferior)
    }


    /// Returns the pid of this inferior.
    pub fn pid(&self) -> Pid {
        nix::unistd::Pid::from_raw(self.child.id() as i32)
    }

    /// Calls waitpid on this inferior and returns a Status to indicate the state of the process
    /// after the waitpid call.
    pub fn wait(&mut self, options: Option<WaitPidFlag>) -> Result<Status, nix::Error> {
        Ok(match waitpid(self.pid(), options)? {
            WaitStatus::Exited(_pid, exit_code) => {
                self.is_exited = true;
                Status::Exited(exit_code)
            },
            WaitStatus::Signaled(_pid, signal, _core_dumped) => Status::Signaled(signal),
            WaitStatus::Stopped(_pid, signal) => {
                let regs = ptrace::getregs(self.pid())?;
                Status::Stopped(signal, regs.rip as usize)
            }
            other => panic!("waitpid returned unexpected status: {:?}", other),
        })
    }

    pub fn continues(&mut self, brk_point_map : &HashMap<usize, u8>) -> Result<Status, nix::Error> {
        if(self.is_brk_point) {
            self.is_brk_point = false;
            ptrace::step(self.pid(), None);
            // todo: figure out the meaning of ptrace::step.
            let status= self.wait(None)?;
            match status.borrow() {
                Status::Stopped(signal, rip) => {
                    self.write_byte(self.brk_point_rip.unwrap(), 0xcc);
                }
                Status::Exited(_) => {
                    return Ok(status);
                }
                _ => {}
            }
        }


        ptrace::cont(self.pid(), None).unwrap();
        let status = self.wait(None)?;

        match status.borrow() {
            Status::Stopped(signal, rip) => {
                let brk_point_rip = rip.clone() - 1;
                let orig_byte_opt = brk_point_map.get(&brk_point_rip);
                match orig_byte_opt {
                    Some(orig_byte) => {
                        self.write_byte(brk_point_rip.clone(), orig_byte.clone());
                        self.is_brk_point = true;

                        // set %rip = %rip - 1
                        let mut regs = ptrace::getregs(self.pid())?;
                        regs.rip = brk_point_rip as u64;
                        self.brk_point_rip = Some(brk_point_rip);
                        ptrace::setregs(self.pid(), regs)?;
                    },
                    None => {}
                }
            }
            _ => {}
        }

        Ok(status)
    }

    pub fn kill(&mut self) {
        let pid = self.pid();
        if self.child.kill().is_ok() {
            self.wait(None).unwrap();
            println!("Killing running inferior (pid {})", pid);
        }
    }

    pub fn print_backtrace(&self, debug_data: &DwarfData) -> Result<(), nix::Error> {
        let user_regs = ptrace::getregs(self.pid())?;
        let mut rbp = user_regs.rbp as usize;
        let mut rip = user_regs.rip as usize;
        let mut line_info;
        let mut func_name;
        while true {
            match debug_data.get_line_from_addr(rip) {
                Some(val) => {
                    line_info = val;
                },
                None => {
                    println!("Couldn't get line from addr.");
                    return Ok(());
                }
            }
            match debug_data.get_function_from_addr(rip) {
                Some(val) => {
                    func_name = val;
                },
                None => {
                    println!("Couldn't get func name from addr.");
                    return Ok(());
                }
            }
            println!("{} ({})", func_name, line_info);
            if func_name == "main" {
                break;
            }

            rip = ptrace::read(self.pid(), (rbp + 8) as ptrace::AddressType)? as usize;
            rbp = ptrace::read(self.pid(), rbp as ptrace::AddressType)? as usize;
        }
        Ok(())
    }

    pub fn write_byte(&self, addr: usize, val: u8) -> Result<u8, nix::Error> {
        let aligned_addr = align_addr_to_word(addr);
        let byte_offset = addr - aligned_addr;
        let word = ptrace::read(self.pid(), aligned_addr as ptrace::AddressType)? as u64;
        let orig_byte = (word >> 8 * byte_offset) & 0xff;
        let masked_word = word & !(0xff << 8 * byte_offset);
        let updated_word = masked_word | ((val as u64) << 8 * byte_offset);
        ptrace::write(
            self.pid(),
            aligned_addr as ptrace::AddressType,
            updated_word as *mut std::ffi::c_void,
        )?;
        Ok(orig_byte as u8)
    }

    pub fn is_exited(&self) -> bool {
        self.is_exited
    }

}

fn align_addr_to_word(addr: usize) -> usize {
    addr & (-(size_of::<usize>() as isize) as usize)
}

