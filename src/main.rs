use std::io::{stdout, Write};

use azuki_opt::{
    branching_simplify::BranchingSimplify, const_folding::ConstFolding,
    dead_code_eliminator::DeadCodeEliminator,
};
use azuki_syntax::{lexer::lexer, parse};
use azuki_tac::optimizer::sanity_checker::SanityChecker;
use azuki_tacvm::Vm;
use clap::Clap;
use opt::Action;
use tracing::{info, trace, warn};
use tracing_subscriber::fmt::format::FmtSpan;

mod opt;

fn main() {
    let opt = opt::Opt::parse();

    tracing_subscriber::FmtSubscriber::builder()
        .with_target(true)
        .with_max_level(opt.log_level)
        .with_writer(std::io::stderr)
        .without_time()
        .init();

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

    info!("Generating IR");
    let mut program = match azuki_tacgen::compile(&program) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{:?}", e);
            return;
        }
    };

    let mut pipeline = azuki_tac::optimizer::Pipeline::new();

    pipeline.add_func_optimizer(SanityChecker::default());
    pipeline.add_func_optimizer(DeadCodeEliminator::default());
    pipeline.add_func_optimizer(BranchingSimplify);
    pipeline.add_func_optimizer(ConstFolding::new());

    let optimizations = opt
        .optimization
        .clone()
        .unwrap_or_else(|| default_opts().iter().map(|x| x.to_string()).collect());

    for optimization in optimizations {
        info!("Running pass `{}`", optimization);
        if !pipeline.run_pass(&mut program, &optimization) {
            warn!("Cannot find pass `{}`", optimization);
        }
    }

    if opt.action == Action::Compile {
        info!("Writing IR into desired output");

        let func_list = program
            .functions
            .drain()
            .map(|(_, x)| x)
            .collect::<Vec<_>>();

        func_list.iter().for_each(|function| {
            writeln!(output, "{}", function).expect("Failed to write to output file");
            writeln!(output).unwrap();
        });
    } else if opt.action == Action::Run {
        info!("Running program in VM");
        let mut vm = Vm::new(&program);
        let entry = opt.entry_point.as_deref().unwrap_or("main");
        let params = opt.params.clone();
        let res = vm.run_func(entry, params);
        if let Some(res) = res {
            println!("{}", res);
        }
    }
}

fn default_opts() -> &'static [&'static str] {
    &[
        "sanity-check",
        "const-folding",
        "branching-simplify",
        "dead-code-eliminator",
        "branching-simplify",
        "dead-code-eliminator",
    ]
}
