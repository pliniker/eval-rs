extern crate blockalloc;
extern crate clap;
extern crate dirs;
extern crate rustyline;
extern crate stickyimmix;

use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::process;

use clap::{App, Arg};

use rustyline::error::ReadlineError;
use rustyline::Editor;

mod arena;
mod error;
mod headers;
mod lexer;
mod memory;
mod parser;
mod pointerops;
mod primitives;
mod printer;
mod safeptr;
mod symbolmap;
mod taggedptr;

use memory::Memory;
use parser::parse;

/*
/// Read a file into a String
fn load_file(filename: &str) -> Result<String, io::Error> {
    let mut contents = String::new();

    File::open(filename)?.read_to_string(&mut contents)?;

    Ok(contents)
}

/// Read an entire file
/// TODO handle errors out of here more consistently
fn read_file(filename: &str) -> Result<(), ()> {
    let contents = load_file(&filename).unwrap_or_else(|err| {
        println!("failed to read file {}: {}", &filename, err);
        process::exit(1);
    });

    let heap = Arena::new(65536);
    let env = Memory::with_heap(&heap);

    match parser::parse(&contents, &env) {
        Ok(ast) => println!("{}", printer::print(&ast)),
        Err(e) => {
            e.print_with_source(&contents);
        }
    }

    Ok(())
}
*/

/// Read a line at a time, printing the input back out
fn read_print_loop() -> Result<(), ReadlineError> {
    // establish a repl input history file path
    let history_file = match dirs::home_dir() {
        Some(mut path) => {
            path.push(".evalrus_history");
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

    let mem = Memory::new();

    // repl
    let mut input_counter = 1;
    loop {
        let readline = reader.readline(&format!("evalrus:{:03}> ", input_counter));
        input_counter += 1;

        match readline {
            // valid input
            Ok(line) => {
                reader.add_history_entry(&line);

                // parse/"read"
                mem.mutate(|view| {
                    match parse(view, &line) {
                        Ok(value) => {
                            /*
                            // eval
                            match eval(value, &mem) {
                            // print
                            Ok(result) => println!("{}", printer::print(&result)),
                            Err(e) => e.print_with_source(&line),
                        } */
                            println!("{}", printer::print(*value));
                            Ok(())
                        },

                        Err(e) => {
                            e.print_with_source(&line);
                            Err(e)
                        },
                    }
                });
            }

            // some kind of program termination condition
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
    /*
    // parse command line argument, an optional filename
    let matches = App::new("Eval-R-Us")
        .about("Evaluate the expressions!")
        .arg(Arg::with_name("filename")
            .help("Optional filename to read in")
            .index(1))
        .get_matches();

    if let Some(filename) = matches.value_of("filename") {
        // if a filename was specified, read it into a String
        read_file(filename).unwrap_or_else(|_err| {
            println!("Error...");
            process::exit(1);
        });
    } else {
        // otherwise begin a repl
        read_print_loop().unwrap_or_else(|err| {
            println!("exited because: {}", err);
            process::exit(0);
        });
    }*/
    read_print_loop().unwrap_or_else(|err| {
        println!("exited because: {}", err);
        process::exit(0);
    });
}
