use clap::Parser;
use halo2_base::gates::{GateChip, GateInstructions, RangeChip, RangeInstructions};
use halo2_base::utils::ScalarField;
use halo2_base::{AssignedValue, Context, QuantumCell::Constant};
use halo2_scaffold::scaffold::cmd::Cli;
use halo2_scaffold::scaffold::run;
use serde::{Deserialize, Serialize};
use std::env::var;

fn bit_slice<F: ScalarField>(
    ctx: &mut Context<F>,
    gate: &GateChip<F>,
    bits: &Vec<AssignedValue<F>>,
    start: usize,
    end: usize,
) -> AssignedValue<F> {
    gate.inner_product(
        ctx,
        (&bits[start..end]).to_vec(),
        (0..(end - start)).map(|i| Constant(gate.pow_of_two[i])),
    )
}

fn state_transition<F: ScalarField>(
    ctx: &mut Context<F>,
    m: &[AssignedValue<F>],
    pc: AssignedValue<F>,
    ap: AssignedValue<F>,
    fp: AssignedValue<F>,
) -> (AssignedValue<F>, AssignedValue<F>, AssignedValue<F>) {
    let lookup_bits =
        var("LOOKUP_BITS").unwrap_or_else(|_| panic!("LOOKUP_BITS not set")).parse().unwrap();
    let range: RangeChip<F> = RangeChip::default(lookup_bits);
    let gate = range.gate();

    let instruction = gate.select_from_idx(ctx, m.to_vec(), pc);
    let instruction_bits = range.gate().num_to_bits(ctx, instruction, 63);
    let off_dst = bit_slice(ctx, gate, &instruction_bits, 0, 16);
    let off_op0 = bit_slice(ctx, gate, &instruction_bits, 16, 32);
    let off_op1 = bit_slice(ctx, gate, &instruction_bits, 32, 48);
    let dst_reg = instruction_bits[48];
    let op0_reg = instruction_bits[49];

    // calculate op0
    let index0 = gate.add(ctx, ap, off_op0);
    let index1 = gate.add(ctx, fp, off_op0);
    let cell0 = gate.select_from_idx(ctx, m.to_vec(), index0);
    let cell1 = gate.select_from_idx(ctx, m.to_vec(), index1);
    let op0 = gate.select(ctx, cell1, cell0, op0_reg);

    // calculate op1 and instruction_size
    let instruction_size =
        gate.select(ctx, Constant(F::from(2)), Constant(F::from(1)), instruction_bits[50]);

    // op1_src = 0
    let index_start = op0;
    // op1_src = 1
    let index_start = gate.select(ctx, pc, index_start, instruction_bits[50]);
    // op1_src = 2
    let index_start = gate.select(ctx, fp, index_start, instruction_bits[51]);
    // op1_src = 4
    let index_start = gate.select(ctx, ap, index_start, instruction_bits[52]);
    let index = gate.add(ctx, index_start, off_op1);
    let op1 = gate.select_from_idx(ctx, m.to_vec(), index);

    // calculate res
    let sum = gate.add(ctx, op0, op1);
    let prod = gate.mul(ctx, op0, op1);
    // res_logic = 0
    let res = op1;
    // res_logic = 1
    let res = gate.select(ctx, sum, res, instruction_bits[53]);
    // res_logic = 2
    let res = gate.select(ctx, prod, res, instruction_bits[54]);

    // calculate dst
    let index_start = gate.select(ctx, fp, ap, dst_reg);
    let index = gate.add(ctx, index_start, off_dst);
    let dst = gate.select_from_idx(ctx, m.to_vec(), index);

    // calculate next_pc
    // pc_update = 0
    let next_pc = gate.add(ctx, pc, instruction_size);
    // pc_update = 1
    let next_pc = gate.select(ctx, res, next_pc, instruction_bits[55]);
    // pc_update = 2
    let rel_loc = gate.add(ctx, pc, res);
    let next_pc = gate.select(ctx, rel_loc, next_pc, instruction_bits[56]);
    // pc_update = 4
    let jump_loc = gate.add(ctx, pc, op1);
    // change if pc_update = 4 and dst != 0
    let dst_is_zero = gate.is_zero(ctx, dst);
    let dst_is_nonzero = gate.sub(ctx, Constant(F::from(1)), dst_is_zero);
    let use_jump_loc = gate.and(ctx, instruction_bits[57], dst_is_nonzero);
    let next_pc = gate.select(ctx, jump_loc, next_pc, use_jump_loc);

    // calculate next_ap, next_fp
    // opcode = 0, 2, 4 case
    // ap_update = 0
    let next_ap = ap;
    // ap_update = 1
    let sum = gate.add(ctx, ap, res);
    let next_ap = gate.select(ctx, sum, next_ap, instruction_bits[58]);
    // ap_update = 2
    let sum = gate.add(ctx, ap, Constant(F::from(1)));
    let next_ap = gate.select(ctx, sum, next_ap, instruction_bits[59]);
    
    let next_fp = fp;
    // opcode = 2
    let next_fp = gate.select(ctx, dst, next_fp, instruction_bits[60]);
    // opcode = 4
    // assert


    // opcode = 1 case
    // asserts
    let sum = gate.add(ctx, ap, Constant(F::from(2)));
    let next_fp = gate.select(ctx, sum, next_fp, instruction_bits[61]);
    let next_ap = gate.select(ctx, sum, next_ap, instruction_bits[61]);

    (next_pc, next_ap, next_fp)
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    // run different zk commands based on the command line arguments
    // run(some_algorithm_in_zk, args);
}
