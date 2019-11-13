extern crate blockalloc;
extern crate clap;
extern crate dirs;
extern crate num;
#[macro_use]
extern crate num_derive;
extern crate rustyline;
extern crate stickyimmix;

use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::process;

use clap::{App, Arg};

use rustyline::error::ReadlineError;
use rustyline::Editor;

mod arena;
mod array;
mod bytecode;
mod compiler;
mod containers;
mod error;
mod headers;
mod lexer;
mod memory;
mod pair;
mod parser;
mod pointerops;
mod primitives;
mod printer;
mod rawarray;
mod safeptr;
mod symbolmap;
mod taggedptr;
mod vm;

use crate::compiler::compile;
use crate::error::{ErrorKind, RuntimeError};
use crate::memory::{Memory, Mutator, MutatorView};
use crate::parser::parse;
use crate::printer::Print;
use crate::vm::quick_vm_eval;

/// Read a file into a String
fn load_file(filename: &str) -> Result<String, io::Error> {
    let mut contents = String::new();

    File::open(filename)?.read_to_string(&mut contents)?;

    Ok(contents)
}

/// Read and evaluate an entire file
fn read_file(filename: &str) -> Result<(), RuntimeError> {
    let contents = load_file(&filename)?;

    // TODO

    Ok(())
}

/// Implements an iteration of a REPL
struct ReadEvalPrint {}

impl Mutator for ReadEvalPrint {
    type Input = String;
    type Output = ();

    fn run(&self, mem: &MutatorView, line: String) -> Result<(), RuntimeError> {
        match parse(mem, &line) {
            Ok(value) => {
                match compile(mem, value) {
                    Ok(result) => {
                        // println!("{}", result);  // prints bytecode
                        let value = quick_vm_eval(mem, result)?;
                        println!("{}", value);
                    }
                    Err(e) => e.print_with_source(&line),
                }

                // println!("{}", printer::print(*value));

                Ok(())
            }

            Err(e) => {
                match e.error_kind() {
                    // non-fatal repl errors
                    ErrorKind::LexerError(_) => e.print_with_source(&line),
                    ErrorKind::ParseError(_) => e.print_with_source(&line),
                    ErrorKind::EvalError(_) => e.print_with_source(&line),
                    _ => return Err(e),
                }
                Ok(())
            }
        }
    }
}

/// Read a line at a time, printing the input back out
fn read_print_loop() -> Result<(), RuntimeError> {
    // establish a repl input history file path
    let history_file = match dirs::home_dir() {
        Some(mut path) => {
            path.push(".evalrus_history");
            Some(String::from(path.to_str().unwrap()))
        }
        None => None,
    };

    // () means no completion support (TODO)
    let mut reader = Editor::<()>::new();

    // Try to load the repl history file
    if let Some(ref path) = history_file {
        if let Err(err) = reader.load_history(&path) {
            eprintln!("Could not read history: {}", err);
        }
    }

    let mem = Memory::new();
    let rep = ReadEvalPrint {};

    // repl
    loop {
        let readline = reader.readline("> ");

        match readline {
            // valid input
            Ok(line) => {
                reader.add_history_entry(&line);
                mem.mutate(&rep, line)?;
            }

            // some kind of program termination condition
            Err(e) => {
                if let Some(ref path) = history_file {
                    reader.save_history(&path).unwrap_or_else(|err| {
                        eprintln!("could not save input history in {}: {}", path, err);
                    });
                }

                // EOF is fine
                if let ReadlineError::Eof = e {
                    return Ok(());
                } else {
                    return Err(RuntimeError::from(e));
                }
            }
        }
    }
}

fn main() {
    // parse command line argument, an optional filename
    let matches = App::new("Eval-R-Us")
        .about("Evaluate the expressions!")
        .arg(
            Arg::with_name("filename")
                .help("Optional filename to read in")
                .index(1),
        )
        .get_matches();

    if let Some(filename) = matches.value_of("filename") {
        // if a filename was specified, read it into a String
        read_file(filename).unwrap_or_else(|err| {
            eprintln!("Terminated: {}", err);
            process::exit(1);
        });
    } else {
        // otherwise begin a repl
        read_print_loop().unwrap_or_else(|err| {
            eprintln!("Terminated: {}", err);
            process::exit(1);
        });
    }
}
