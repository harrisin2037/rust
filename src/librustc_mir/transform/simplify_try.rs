//! The general point of the optimizations provided here is to simplify something like:
//!
//! ```rust
//! match x {
//!     Ok(x) => Ok(x),
//!     Err(x) => Err(x)
//! }
//! ```
//!
//! into just `x`.

use crate::transform::{simplify, MirPass, MirSource};
use itertools::Itertools as _;
use rustc::mir::*;
use rustc::ty::{Ty, TyCtxt};
use rustc_index::vec::IndexVec;
use rustc_target::abi::VariantIdx;

/// Simplifies arms of form `Variant(x) => Variant(x)` to just a move.
///
/// This is done by transforming basic blocks where the statements match:
///
/// ```rust
/// _LOCAL_TMP = ((_LOCAL_1 as Variant ).FIELD: TY );
/// _TMP_2 = _LOCAL_TMP;
/// ((_LOCAL_0 as Variant).FIELD: TY) = move _TMP_2;
/// discriminant(_LOCAL_0) = VAR_IDX;
/// ```
///
/// into:
///
/// ```rust
/// _LOCAL_0 = move _LOCAL_1
/// ```
pub struct SimplifyArmIdentity;

#[derive(Debug)]
struct ArmIdentityInfo<'tcx> {
    /// Storage location for the variant's field
    local_temp_0: Local,
    /// Storage location holding the varient being read from
    local_1: Local,
    /// The varient field being read from
    vf_s0: VarField<'tcx>,

    /// Tracks each assignment to a temporary of the varient's field
    field_tmp_assignments: Vec<(Local, Local)>,

    /// Storage location holding the variant's field that was read from
    local_tmp_s1: Local,
    /// Storage location holding the enum that we are writing to
    local_0: Local,
    /// The varient field being written to
    vf_s1: VarField<'tcx>,

    /// Storage location that the discrimentant is being set to
    set_discr_local: Local,
    /// The variant being written
    set_discr_var_idx: VariantIdx,

    /// Index of the statement that should be overwritten as a move
    stmt_to_overwrite: usize,
    /// SourceInfo for the new move
    source_info: SourceInfo,

    /// Indexes of matching Storage{Live,Dead} statements encountered.
    /// (StorageLive index,, StorageDead index, Local)
    storage_stmts: Vec<(usize, usize, Local)>,

    /// The statements that should be removed (turned into nops)
    stmts_to_remove: Vec<usize>,
}

fn get_arm_identity_info(stmts: &[Statement<'tcx>]) -> Option<ArmIdentityInfo<'tcx>> {
    let (mut local_tmp_s0, mut local_1, mut vf_s0) = (None, None, None);
    let mut tmp_assigns = Vec::new();
    let (mut local_tmp_s1, mut local_0, mut vf_s1) = (None, None, None);
    let (mut set_discr_local, mut set_discr_var_idx) = (None, None);
    let mut starting_stmt = None;
    let mut discr_stmt = None;
    let mut nop_stmts = Vec::new();
    let mut storage_stmts = Vec::new();
    let mut storage_live_stmts = Vec::new();
    let mut storage_dead_stmts = Vec::new();

    for (stmt_idx, stmt) in stmts.iter().enumerate() {
        if let StatementKind::StorageLive(l) = stmt.kind {
            storage_live_stmts.push((stmt_idx, l));
            continue;
        } else if let StatementKind::StorageDead(l) = stmt.kind {
            storage_dead_stmts.push((stmt_idx, l));
            continue;
        }

        if local_tmp_s0 == None && local_1 == None && vf_s0 == None {
            let result = match_get_variant_field(stmt)?;
            local_tmp_s0 = Some(result.0);
            local_1 = Some(result.1);
            vf_s0 = Some(result.2);
            starting_stmt = Some(stmt_idx);
        } else if let StatementKind::Assign(box (place, Rvalue::Use(op))) = &stmt.kind {
            if let Some(local) = place.as_local() {
                if let Operand::Copy(p) | Operand::Move(p) = op {
                    tmp_assigns.push((local, p.as_local()?));
                    nop_stmts.push(stmt_idx);
                } else {
                    return None;
                }
            } else if local_tmp_s1 == None && local_0 == None && vf_s1 == None {
                let result = match_set_variant_field(stmt)?;
                local_tmp_s1 = Some(result.0);
                local_0 = Some(result.1);
                vf_s1 = Some(result.2);
                nop_stmts.push(stmt_idx);
            }
        } else if set_discr_local == None && set_discr_var_idx == None {
            let result = match_set_discr(stmt)?;
            set_discr_local = Some(result.0);
            set_discr_var_idx = Some(result.1);
            discr_stmt = Some(stmt);
            nop_stmts.push(stmt_idx);
        }
    }

    for (live_idx, live_local) in storage_live_stmts {
        if let Some(i) = storage_dead_stmts.iter().rposition(|(_, l)| *l == live_local) {
            let (dead_idx, _) = storage_dead_stmts.swap_remove(i);
            storage_stmts.push((live_idx, dead_idx, live_local));
        }
    }

    Some(ArmIdentityInfo {
        local_temp_0: local_tmp_s0?,
        local_1: local_1?,
        vf_s0: vf_s0?,
        field_tmp_assignments: tmp_assigns,
        local_tmp_s1: local_tmp_s1?,
        local_0: local_0?,
        vf_s1: vf_s1?,
        set_discr_local: set_discr_local?,
        set_discr_var_idx: set_discr_var_idx?,
        stmt_to_overwrite: starting_stmt?,
        source_info: discr_stmt?.source_info,
        storage_stmts: storage_stmts,
        stmts_to_remove: nop_stmts,
    })
}

fn optimization_applies<'tcx>(opt_info: &ArmIdentityInfo<'tcx>, local_decls: &IndexVec<Local, LocalDecl<'tcx>>) -> bool {
    trace!("testing if optimization applies...");

    if opt_info.local_0 == opt_info.local_1 {
        trace!("NO: moving into ourselves");
        return false;
    } else if opt_info.vf_s0 != opt_info.vf_s1 {
        trace!("NO: the field-and-variant information do not match");
        return false;
    } else if local_decls[opt_info.local_0].ty != local_decls[opt_info.local_1].ty {
        // FIXME(Centril,oli-obk): possibly relax ot same layout?
        trace!("NO: source and target locals have different types");
        return false;
    } else if (opt_info.local_0, opt_info.vf_s0.var_idx) != (opt_info.set_discr_local, opt_info.set_discr_var_idx) {
        trace!("NO: the discriminants do not match");
        return false;
    }

    // Verify the assigment chain consists of the form b = a; c = b; d = c; etc...
    if opt_info.field_tmp_assignments.len() == 0 {
        trace!("NO: no assignments found");
    }
    let mut last_assigned_to = opt_info.field_tmp_assignments[0].1;
    let source_local = last_assigned_to;
    for (l, r) in &opt_info.field_tmp_assignments {
        if *r != last_assigned_to {
            trace!("NO: found unexpected assignment {:?} = {:?}", l, r);
            return false;
        }

        last_assigned_to = *l;
    }

    if source_local != opt_info.local_temp_0 {
        trace!("NO: start of assignment chain does not match enum variant temp: {:?} != {:?}", source_local, opt_info.local_temp_0);
        return false;
    } else if last_assigned_to != opt_info.local_tmp_s1 {
        trace!("NO: end of assignemnt chain does not match written enum temp: {:?} != {:?}", last_assigned_to, opt_info.local_tmp_s1);
        return false;
    }

    trace!("SUCCESS: optimization applies!");
    return true;
}

impl<'tcx> MirPass<'tcx> for SimplifyArmIdentity {
    fn run_pass(&self, _: TyCtxt<'tcx>, source: MirSource<'tcx>, body: &mut BodyAndCache<'tcx>) {
        trace!("running SimplifyArmIdentity on {:?}", source);
        let (basic_blocks, local_decls) = body.basic_blocks_and_local_decls_mut();
        for bb in basic_blocks {
            trace!("bb.len() = {:?}", bb.statements.len());

            if let Some(mut opt_info) = get_arm_identity_info(&bb.statements) {
                trace!("got opt_info = {:#?}", opt_info);
                if !optimization_applies(&opt_info, local_decls) {
                    debug!("skipping simplification!!!!!!!!!!!");
                    continue;
                }

                trace!("proceeding...");

                //if tcx.sess.opts.debugging_opts.mir_opt_level <= 1 {
                //    continue;
                //}

                // Also remove unused Storage{Live,Dead} statements which correspond
                // to temps used previously.
                for (left, right) in opt_info.field_tmp_assignments {
                    for (live_idx, dead_idx, local) in &opt_info.storage_stmts {
                        if *local == left || *local == right {
                            opt_info.stmts_to_remove.push(*live_idx);
                            opt_info.stmts_to_remove.push(*dead_idx);
                        }
                    }
                }

                // Right shape; transform!
                let stmt = &mut bb.statements[opt_info.stmt_to_overwrite];
                stmt.source_info = opt_info.source_info;
                match &mut stmt.kind {
                    StatementKind::Assign(box (place, rvalue)) => {
                        *place = opt_info.local_0.into();
                        *rvalue = Rvalue::Use(Operand::Move(opt_info.local_1.into()));
                    }
                    _ => unreachable!(),
                }

                for stmt_idx in opt_info.stmts_to_remove {
                    bb.statements[stmt_idx].make_nop();
                }

                bb.statements.retain(|stmt| stmt.kind != StatementKind::Nop);

                trace!("block is now {:?}", bb.statements);
            }
        }
    }
}

/// Match on:
/// ```rust
/// _LOCAL_INTO = ((_LOCAL_FROM as Variant).FIELD: TY);
/// ```
fn match_get_variant_field<'tcx>(stmt: &Statement<'tcx>) -> Option<(Local, Local, VarField<'tcx>)> {
    match &stmt.kind {
        StatementKind::Assign(box (place_into, rvalue_from)) => match rvalue_from {
            Rvalue::Use(Operand::Copy(pf)) | Rvalue::Use(Operand::Move(pf)) => {
                let local_into = place_into.as_local()?;
                let (local_from, vf) = match_variant_field_place(&pf)?;
                Some((local_into, local_from, vf))
            }
            _ => None,
        },
        _ => None,
    }
}

/// Match on:
/// ```rust
/// ((_LOCAL_FROM as Variant).FIELD: TY) = move _LOCAL_INTO;
/// ```
fn match_set_variant_field<'tcx>(stmt: &Statement<'tcx>) -> Option<(Local, Local, VarField<'tcx>)> {
    match &stmt.kind {
        StatementKind::Assign(box (place_from, rvalue_into)) => match rvalue_into {
            Rvalue::Use(Operand::Move(place_into)) => {
                let local_into = place_into.as_local()?;
                let (local_from, vf) = match_variant_field_place(&place_from)?;
                Some((local_into, local_from, vf))
            }
            _ => None,
        },
        _ => None,
    }
}

/// Match on:
/// ```rust
/// discriminant(_LOCAL_TO_SET) = VAR_IDX;
/// ```
fn match_set_discr<'tcx>(stmt: &Statement<'tcx>) -> Option<(Local, VariantIdx)> {
    match &stmt.kind {
        StatementKind::SetDiscriminant { place, variant_index } => {
            Some((place.as_local()?, *variant_index))
        }
        _ => None,
    }
}

#[derive(PartialEq, Debug)]
struct VarField<'tcx> {
    field: Field,
    field_ty: Ty<'tcx>,
    var_idx: VariantIdx,
}

/// Match on `((_LOCAL as Variant).FIELD: TY)`.
fn match_variant_field_place<'tcx>(place: &Place<'tcx>) -> Option<(Local, VarField<'tcx>)> {
    match place.as_ref() {
        PlaceRef {
            local,
            projection: &[ProjectionElem::Downcast(_, var_idx), ProjectionElem::Field(field, ty)],
        } => Some((local, VarField { field, field_ty: ty, var_idx })),
        _ => None,
    }
}

/// Simplifies `SwitchInt(_) -> [targets]`,
/// where all the `targets` have the same form,
/// into `goto -> target_first`.
pub struct SimplifyBranchSame;

impl<'tcx> MirPass<'tcx> for SimplifyBranchSame {
    fn run_pass(&self, _: TyCtxt<'tcx>, _: MirSource<'tcx>, body: &mut BodyAndCache<'tcx>) {
        let mut did_remove_blocks = false;
        let bbs = body.basic_blocks_mut();
        for bb_idx in bbs.indices() {
            let targets = match &bbs[bb_idx].terminator().kind {
                TerminatorKind::SwitchInt { targets, .. } => targets,
                _ => continue,
            };

            let mut iter_bbs_reachable = targets
                .iter()
                .map(|idx| (*idx, &bbs[*idx]))
                .filter(|(_, bb)| {
                    // Reaching `unreachable` is UB so assume it doesn't happen.
                    bb.terminator().kind != TerminatorKind::Unreachable
                    // But `asm!(...)` could abort the program,
                    // so we cannot assume that the `unreachable` terminator itself is reachable.
                    // FIXME(Centril): use a normalization pass instead of a check.
                    || bb.statements.iter().any(|stmt| match stmt.kind {
                        StatementKind::InlineAsm(..) => true,
                        _ => false,
                    })
                })
                .peekable();

            // We want to `goto -> bb_first`.
            let bb_first = iter_bbs_reachable.peek().map(|(idx, _)| *idx).unwrap_or(targets[0]);

            // All successor basic blocks should have the exact same form.
            let all_successors_equivalent =
                iter_bbs_reachable.map(|(_, bb)| bb).tuple_windows().all(|(bb_l, bb_r)| {
                    bb_l.is_cleanup == bb_r.is_cleanup
                        && bb_l.terminator().kind == bb_r.terminator().kind
                        && bb_l.statements.iter().eq_by(&bb_r.statements, |x, y| x.kind == y.kind)
                });

            if all_successors_equivalent {
                // Replace `SwitchInt(..) -> [bb_first, ..];` with a `goto -> bb_first;`.
                bbs[bb_idx].terminator_mut().kind = TerminatorKind::Goto { target: bb_first };
                did_remove_blocks = true;
            }
        }

        if did_remove_blocks {
            // We have dead blocks now, so remove those.
            simplify::remove_dead_blocks(body);
        }
    }
}
