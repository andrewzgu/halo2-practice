use clap::Parser;
use halo2_base::gates::{GateInstructions, RangeChip, RangeInstructions};
use halo2_base::QuantumCell;
use halo2_base::QuantumCell::{Constant, Existing};
use halo2_base::{utils::ScalarField, AssignedValue, Context};
use halo2_scaffold::scaffold::{cmd::Cli, run};
use serde::{Deserialize, Serialize};
use std::env::var;

const ARR_SIZE: usize = 10;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput {
    pub arr: [String; ARR_SIZE],
    pub start: String,
    pub end: String,
}

fn subarray<F: ScalarField>(
    ctx: &mut Context<F>,
    input: CircuitInput,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    // note: hardcoded, should satisfy 2^{NUM_BITS} > ARR_SIZE
    const NUM_BITS: usize = 5;

    let arr = input.arr.map(|x| ctx.load_witness(F::from_str_vartime(&x).unwrap()));
    let start =
        F::from_str_vartime(&input.start).expect("deserialize field element should not fail");
    let end = F::from_str_vartime(&input.end).expect("deserialize field element should not fail");
    let start = ctx.load_witness(start);
    let end = ctx.load_witness(end);
    make_public.extend(arr);

    let lookup_bits =
        var("LOOKUP_BITS").unwrap_or_else(|_| panic!("LOOKUP_BITS not set")).parse().unwrap();
    let range = RangeChip::default(lookup_bits);
    // check inequality constratints for start and end
    range.check_less_than_safe(ctx, start, ARR_SIZE as u64);
    range.check_less_than_safe(ctx, end, (ARR_SIZE + 1) as u64);

    // with [start, end) we may have start = end to indicate an empty selection
    let end_plus_one = range.gate().add(ctx, end, Constant(F::from(1)));
    range.check_less_than(ctx, start, end_plus_one, NUM_BITS);

    let mut arr_cells: Vec<QuantumCell<F>> = arr.map(|x| Existing(x)).into_iter().collect();
    // construct the array arr[start:] with barrel shifter
    let start_bits = range.gate().num_to_bits(ctx, start, NUM_BITS);
    for lvl in 0..NUM_BITS {
        // if start has 2^i in binary representation, (cyclically) shift left by 2^i
        let mut shifted_arr: Vec<QuantumCell<F>> = Vec::with_capacity(ARR_SIZE);
        let bit = start_bits[lvl];
        for i in 0..ARR_SIZE {
            shifted_arr.push(Existing(range.gate().select(
                ctx,
                arr_cells[(i + (1 << lvl)) % ARR_SIZE],
                arr_cells[i],
                bit,
            )));
        }
        arr_cells = shifted_arr;
    }

    let mut out_arr: Vec<AssignedValue<F>> = Vec::with_capacity(ARR_SIZE);
    for i in 0..ARR_SIZE {
        // arr_index = index that goes into arr, if valid
        let arr_index = range.gate().add(ctx, start, Constant(F::from(i as u64)));
        // valid_selection = bool that indicates whether we are on the prefix with part of the
        // subarray
        let valid_selection = range.is_less_than(ctx, arr_index, end, NUM_BITS);

        // if invalid, select 0 instead
        out_arr.push(range.gate().select(ctx, arr_cells[i], Constant(F::from(0)), valid_selection));
    }

    for i in 0..ARR_SIZE {
        println!("out[{}]: {:?}", i, out_arr[i].value());
    }
}

fn main() {
    env_logger::init();

    let args = Cli::parse();
    run(subarray, args);
}
