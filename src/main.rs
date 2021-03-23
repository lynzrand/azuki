use std::io::{stdout, Write};

use azuki_opt::dead_code_eliminator::DeadCodeEliminator;
use azuki_syntax::{lexer::lexer, parse};
use azuki_tac::optimizer::sanity_checker::SanityChecker;
use azuki_tacvm::Vm;
use clap::Clap;
use opt::Action;

mod opt;

fn main() {
    let opt = opt::Opt::parse();

    let file = opt.file;
    let input = std::fs::read_to_string(file).expect("Unable to read input file");

    let mut output: Box<dyn Write> = match opt.out_file {
        Some(file) => Box::new(
            std::fs::OpenOptions::new()
                .write(true)
                .open(file)
                .expect("Failed to open output file"),
        ),
        None => Box::new(stdout()),
    };

    if opt.action == Action::Lex {
        // lex file
        let lexer = lexer(&input);
        lexer.for_each(|token| {
            writeln!(output, "{}", token).expect("Failed to write to output file")
        });
        return;
    }

    let program = match parse(&input) {
        Ok(p) => p,
        Err(e) => {
            // TODO: Error display
            println!("{:?}", e);
            return;
        }
    };

    if opt.action == Action::Parse {
        // TODO: output parse result
        return;
    }

    let mut program = match azuki_tacgen::compile(&program) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{:?}", e);
            return;
        }
    };

    let mut pipeline = azuki_tac::optimizer::Pipeline::new();

    pipeline.add_func_optimizer(SanityChecker::default());
    pipeline.add_func_optimizer(DeadCodeEliminator::new());

    for optimization in &opt.optimization {
        pipeline.run_pass(&mut program, optimization);
    }

    if opt.action == Action::Compile {
        program.functions.iter().for_each(|(_, function)| {
            writeln!(output, "{}", function).expect("Failed to write to output file");
            writeln!(output).unwrap();
        });
    } else if opt.action == Action::Run {
        // TODO: Run program
        let vm = Vm::new(&program);
    }
}
