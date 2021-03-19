use clap::{App, Arg};
use dol_cxx_patch_generator::generate_patches;
use dol_linker::read_symbol_table;

fn main() {
    let matches = App::new("dol_cxx_patch_generator")
        .arg(
            Arg::with_name("dol")
                .long("dol")
                .short("d")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("symbol-map")
                .long("symbol-map")
                .short("s")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("output")
                .long("output")
                .short("o")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("patch")
                .multiple(true)
                .min_values(1)
                .required(true),
        )
        .get_matches();

    let extern_sym_table_fname = matches.value_of("symbol-map").unwrap();

    let extern_sym_table = read_symbol_table(extern_sym_table_fname).unwrap();
    generate_patches(
        &extern_sym_table,
        matches.value_of("dol").unwrap(),
        matches.value_of("output").unwrap(),
        matches.values_of("patch").unwrap(),
    )
    .unwrap();
}
