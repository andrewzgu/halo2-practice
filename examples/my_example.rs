use clap::Parser;
use halo2_base::gates::{GateChip, GateInstructions, RangeChip, RangeInstructions};
use halo2_base::utils::ScalarField;
use halo2_base::AssignedValue;
use halo2_base::{
    Context,
    QuantumCell::{Constant, Existing, Witness},
};
use halo2_scaffold::scaffold::cmd::Cli;
use halo2_scaffold::scaffold::run;
use serde::{Deserialize, Serialize};
use std::env::var;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput {
    pub x: String, // field element, but easier to deserialize as a string
}

// this algorithm takes a public input x, computes x^2 + 72, and outputs the result as public output
fn some_algorithm_in_zk<F: ScalarField>(
    ctx: &mut Context<F>,
    input: CircuitInput,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    let x = F::from_str_vartime(&input.x).expect("deserialize field element should not fail");
    // `Context` can roughly be thought of as a single-threaded execution trace of a program we want to ZK prove. We do some post-processing on `Context` to optimally divide the execution trace into multiple columns in a PLONKish arithmetization
    // More advanced usage with multi-threaded witness generation is possible, but we do not explain it here

    // first we load a number `x` into as system, as a "witness"
    let x = ctx.load_witness(x);
    // by default, all numbers in the system are private
    // we can make it public like so:
    make_public.push(x);

    let lookup_bits =
        var("LOOKUP_BITS").unwrap_or_else(|_| panic!("LOOKUP_BITS not set")).parse().unwrap();
    let range = RangeChip::default(lookup_bits);
    const BYTE_SIZE: usize = 8;
    const INPUT_BYTES: usize = 4;
    const OUTPUT_BYTES: usize = 2;

    //range.range_check(ctx, x, INPUT_BYTES * BYTE_SIZE);
    let x_bytes = x.value().to_repr();
    let bytes = x_bytes.as_ref().iter().map(|x| Witness(F::from(*x as u64))).take(INPUT_BYTES);

    let row_offset = ctx.advice.len();
    // Check that the sum of the decomposition parts is right
    let claimed_val = range.gate().inner_product(
        ctx,
        bytes,
        (0..INPUT_BYTES).map(|i| Constant(range.gate().pow_of_two[8 * i])),
    );

    ctx.constrain_equal(&x, &claimed_val);

    let mut byte_cells = Vec::with_capacity(INPUT_BYTES);
    byte_cells.push(ctx.get(row_offset as isize));
    for i in 1..INPUT_BYTES {
        byte_cells.push(ctx.get((row_offset + 1 + 3 * (i - 1)) as isize));
    }

    // Check that these values are byte size
    for cell in byte_cells.iter() {
        range.range_check(ctx, *cell, BYTE_SIZE);
    }

    // Compute the output from the bottom bytes
    let out = range.gate().inner_product(
        ctx,
        byte_cells.iter().map(|value| Existing(*value)).take(OUTPUT_BYTES),
        (0..OUTPUT_BYTES).map(|i| Constant(range.gate().pow_of_two[8 * i])),
    );
    println!("x: {:?}", x.value());
    println!("out: {:?}", out.value());
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    // run different zk commands based on the command line arguments
    run(some_algorithm_in_zk, args);
}
