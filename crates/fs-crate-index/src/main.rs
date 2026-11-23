//! Pattern: none (thin binary: hands the command line to the library and exits with its code).

fn main() {
    std::process::exit(fs_crate_index::main_with(std::env::args().collect()));
}
