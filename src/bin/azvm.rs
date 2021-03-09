use std::{cell::RefCell, path::PathBuf, process::exit, rc::Rc};

use azuki_tac::parser::parse_program_from_string;
use azuki_tacvm::{inspector::Inspector, Vm};
use clap::Clap;

#[derive(Clap, Debug)]
struct Opt {
    file: PathBuf,

    #[clap(long = "debug")]
    debug: bool,

    #[clap(long, default_value = "main")]
    entry_point: String,

    #[clap(long)]
    entry_params: Vec<i64>,

    #[clap(long = "inst-count")]
    instruction_count: bool,
}

fn main() {
    let opt = Opt::parse();
    let program = match std::fs::read_to_string(&opt.file) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to open file {}: {}", &opt.file.to_string_lossy(), e);
            exit(1);
        }
    };
    let program = match parse_program_from_string(&program) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Input is not a valid Azuki TAC file.");
            eprintln!();
            eprintln!("{}", e);
            exit(2);
        }
    };
    let mut vm = Vm::new(&program);

    let inst_cnt = if opt.instruction_count {
        let x = Rc::new(RefCell::new(InstCounter(0)));
        vm.add_inspector_boxed(x.clone());
        Some(x)
    } else {
        None
    };

    vm.run_func(&opt.entry_point, opt.entry_params);

    if let Some(inst_cnt) = inst_cnt {
        eprintln!("Instruction count: {}", inst_cnt.borrow().0);
    }
}

struct InstCounter(usize);

impl Inspector for InstCounter {
    fn before_inst(&mut self, inst: &azuki_tac::Inst, _frame: &azuki_tacvm::Frame) {
        match &inst.kind {
            azuki_tac::InstKind::FunctionCall(c) => self.0 += (c.params.len() + 1) * 2,
            _ => self.0 += 1,
        }
    }

    fn before_branch(&mut self, _inst: &azuki_tac::Branch, _frame: &azuki_tacvm::Frame) {}

    fn before_call(&mut self, _params: &[i64], _func: &azuki_tac::TacFunc) {}

    fn before_ret(&mut self, _frame: &azuki_tacvm::Frame) {}
}
