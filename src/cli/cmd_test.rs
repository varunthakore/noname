use miette::{IntoDiagnostic, Result, WrapErr};
use std::path::PathBuf;

use crate::{
    inputs::{parse_inputs, JsonInputs},
    prover::compile_and_prove,
};

#[derive(clap::Parser)]
pub struct CmdTest {
    /// path to the .no file
    #[clap(short, long, value_parser)]
    path: PathBuf,

    /// public inputs in a JSON format using decimal values (e.g. {"a": "1", "b": "2"})
    #[clap(long)]
    public_inputs: Option<String>,

    /// private inputs in a JSON format using decimal values (e.g. {"a": "1", "b": "2"})
    #[clap(long)]
    private_inputs: Option<String>,

    /// prints debug information (defaults to false)
    #[clap(short, long)]
    debug: bool,
}

pub fn cmd_test(args: CmdTest) -> Result<()> {
    let code = std::fs::read_to_string(&args.path)
        .into_diagnostic()
        .wrap_err_with(|| "could not read file".to_string())?;

    let public_inputs = if let Some(s) = args.public_inputs {
        parse_inputs(&s)?
    } else {
        JsonInputs::default()
    };

    let private_inputs = if let Some(s) = args.private_inputs {
        parse_inputs(&s)?
    } else {
        JsonInputs::default()
    };

    parse(&code, public_inputs, private_inputs, args.debug).map_err(|e| e.with_source_code(code))
}

fn parse(
    code: &str,
    public_inputs: JsonInputs,
    private_inputs: JsonInputs,
    debug: bool,
) -> Result<()> {
    // compile
    let (prover_index, verifier_index) = compile_and_prove(code)?;
    println!("successfuly compiled");

    // print ASM
    let asm = prover_index.asm(debug);
    println!("{asm}");

    // create proof
    let (proof, full_public_inputs, _public_output) =
        prover_index.prove(public_inputs, private_inputs, debug)?;
    println!("proof created");

    // verify proof
    verifier_index.verify(full_public_inputs, proof)?;
    println!("proof verified");

    Ok(())
}