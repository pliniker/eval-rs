
extern crate clap;
extern crate memalloc;
extern crate rustyline;


use std::env;
use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::process;

use clap::{Arg, App};

use rustyline::error::ReadlineError;
use rustyline::Editor;

mod error;
mod lexer;
mod memory;
mod parser;
mod printer;
mod types;


// read a file into a String
fn load_file(filename: &str) -> Result<String, io::Error> {
    let mut contents = String::new();

    File::open(filename)?.read_to_string(&mut contents)?;

    Ok(contents)
}


// read a line at a time, printing the input back out
fn read_print_loop(mem: &mut memory::Arena) -> Result<(), ReadlineError> {

    // establish a repl input history file path
    let history_file = match env::home_dir() {
        Some(mut path) => {
            path.push(".eval-rs_history");
            Some(String::from(path.to_str().unwrap()))
        }
        None => None,
    };

    // () means no completion support
    let mut reader = Editor::<()>::new();

    // try to load the history file, failing silently if it can't be read
    if let Some(ref path) = history_file {
        if let Err(_) = reader.load_history(&path) { /* ignore absence or unreadability */ }
    }

    // repl
    let mut input_counter = 1;
    loop {
        let readline = reader.readline(&format!("evalrus:{:03}> ", input_counter));
        input_counter += 1;

        match readline {
            // valid input
            Ok(line) => {
                reader.add_history_entry(&line);

                match parser::parse(mem, line) {
                    Ok(ast) => println!("{}", printer::print(&ast)),
                    Err(e) => {
                        println!("Error on line/char {}/{}: {}",
                                 e.lineno(),
                                 e.charno(),
                                 e.message())
                    }
                }
            }

            // some kind of termination condition
            Err(e) => {
                if let Some(ref path) = history_file {
                    reader.save_history(&path).unwrap_or_else(|err| {
                        println!("could not save input history in {}: {}", path, err);
                    });
                }

                return Err(e);
            }
        }
    }
}


fn main() {
    // parse command line argument, an optional filename
    let matches = App::new("Eval-R-Us")
        .about("Evaluate the Expressions!")
        .arg(Arg::with_name("filename")
            .help("Optional filename to read in")
            .index(1))
        .get_matches();

    // make a memory heap
    let mut mem = memory::Arena::new(65536);

    if let Some(filename) = matches.value_of("filename") {
        // if a filename was specified, read it into a String

        let contents = load_file(&filename).unwrap_or_else(|err| {
            println!("failed to read file {}: {}", &filename, err);
            process::exit(1);
        });

        match parser::parse(&mut mem, contents) {
            Ok(ast) => println!("{}", printer::print(&ast)),
            Err(e) => {
                println!("Error on line/char {}/{}: {}",
                         e.lineno(),
                         e.charno(),
                         e.message())
            }
        }

    } else {
        // otherwise begin a repl

        read_print_loop(&mut mem).unwrap_or_else(|err| {
            println!("exited because: {}", err);
            process::exit(0);
        });
    }
}
