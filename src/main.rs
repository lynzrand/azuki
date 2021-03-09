use clap::Clap;

mod opt;

fn main() {
    let opt = opt::Opt::parse();
    println!("{:?}", opt);
}
