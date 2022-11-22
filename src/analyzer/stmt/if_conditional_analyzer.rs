use std::collections::BTreeMap;

use crate::{
    expression_analyzer,
    scope_analyzer::ScopeAnalyzer,
    scope_context::{if_scope::IfScope, ScopeContext},
};
use hakana_reflection_info::{
    data_flow::{graph::GraphKind, node::DataFlowNode, path::PathKind},
    issue::{Issue, IssueKind},
    t_union::TUnion,
};
use oxidized::{aast, ast, ast_defs::Pos};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{reconciler::reconciler, statements_analyzer::StatementsAnalyzer, typed_ast::TastInfo};

use super::if_conditional_scope::IfConditionalScope;

pub(crate) fn analyze<'a>(
    statements_analyzer: &StatementsAnalyzer,
    cond: &aast::Expr<(), ()>,
    tast_info: &mut TastInfo,
    outer_context: &ScopeContext,
    if_scope: &mut IfScope,
) -> IfConditionalScope {
    let mut outer_context = outer_context.clone();
    let mut old_outer_context = outer_context.clone();
    let mut has_outer_context_changes = false;

    if !if_scope.negated_clauses.is_empty() {
        let mut changed_var_ids = FxHashSet::default();

        if !if_scope.negated_types.is_empty() {
            let mut tmp_context = outer_context.clone();

            reconciler::reconcile_keyed_types(
                &if_scope.negated_types,
                BTreeMap::new(),
                &mut tmp_context,
                &mut changed_var_ids,
                &FxHashSet::default(),
                statements_analyzer,
                tast_info,
                cond.pos(),
                true,
                false,
                &FxHashMap::default(),
            );

            if !changed_var_ids.is_empty() {
                outer_context = tmp_context;
                has_outer_context_changes = true;
            }
        }
    }

    // get the first expression in the if, which should be evaluated on its own
    // this allows us to update the context of $matches in
    // if (!preg_match('/a/', 'aa', $matches)) {
    //   exit
    // }
    // echo $matches[0];
    let externally_applied_if_cond_expr = get_definitely_evaluated_expression_after_if(cond);
    let internally_applied_if_cond_expr = get_definitely_evaluated_expression_inside_if(cond);

    let mut if_context = None;

    if externally_applied_if_cond_expr != internally_applied_if_cond_expr {
        if_context = Some(
            if has_outer_context_changes {
                &outer_context
            } else {
                &old_outer_context
            }
            .clone(),
        );
    }

    let pre_condition_vars_in_scope = if has_outer_context_changes {
        &outer_context
    } else {
        &old_outer_context
    }
    .vars_in_scope
    .clone();

    let pre_referenced_var_ids = if has_outer_context_changes {
        &outer_context
    } else {
        &old_outer_context
    }
    .cond_referenced_var_ids
    .clone();

    if has_outer_context_changes {
        &mut outer_context
    } else {
        &mut old_outer_context
    }
    .cond_referenced_var_ids = FxHashSet::default();

    let pre_assigned_var_ids = if has_outer_context_changes {
        &outer_context
    } else {
        &old_outer_context
    }
    .assigned_var_ids
    .clone();

    if has_outer_context_changes {
        &mut outer_context
    } else {
        &mut old_outer_context
    }
    .assigned_var_ids = FxHashMap::default();

    let was_inside_conditional = outer_context.inside_conditional;

    outer_context.inside_conditional = true;

    if !expression_analyzer::analyze(
        statements_analyzer,
        externally_applied_if_cond_expr,
        tast_info,
        if has_outer_context_changes {
            &mut outer_context
        } else {
            &mut old_outer_context
        },
        &mut None,
    ) {
        // do something here
    }

    let first_cond_assigned_var_ids = if has_outer_context_changes {
        &outer_context
    } else {
        &old_outer_context
    }
    .assigned_var_ids
    .clone();

    if has_outer_context_changes {
        &mut outer_context
    } else {
        &mut old_outer_context
    }
    .assigned_var_ids
    .extend(pre_assigned_var_ids);

    let first_cond_referenced_var_ids = if has_outer_context_changes {
        &outer_context
    } else {
        &old_outer_context
    }
    .cond_referenced_var_ids
    .clone();

    if has_outer_context_changes {
        &mut outer_context
    } else {
        &mut old_outer_context
    }
    .cond_referenced_var_ids
    .extend(pre_referenced_var_ids);

    if has_outer_context_changes {
        &mut outer_context
    } else {
        &mut old_outer_context
    }
    .inside_conditional = was_inside_conditional;

    let mut if_context = if let Some(if_context) = if_context {
        Some(if_context)
    } else {
        Some(
            if has_outer_context_changes {
                &outer_context
            } else {
                &old_outer_context
            }
            .clone(),
        )
    };

    let mut if_conditional_context = if_context.clone().unwrap();

    // we need to clone the current context so our ongoing updates
    // to $outer_context don't mess with elseif/else blocks
    let post_if_context = if has_outer_context_changes {
        &outer_context
    } else {
        &old_outer_context
    }
    .clone();

    let mut cond_referenced_var_ids;
    let assigned_in_conditional_var_ids;

    if internally_applied_if_cond_expr != cond || externally_applied_if_cond_expr != cond {
        if_conditional_context.assigned_var_ids = FxHashMap::default();
        if_conditional_context.cond_referenced_var_ids = FxHashSet::default();

        let was_inside_conditional = if_conditional_context.inside_conditional;

        if_conditional_context.inside_conditional = true;

        if !expression_analyzer::analyze(
            statements_analyzer,
            cond,
            tast_info,
            &mut if_conditional_context,
            &mut if_context,
        ) {
            // do something here
        }

        add_branch_dataflow(statements_analyzer, cond, tast_info);

        if_conditional_context.inside_conditional = was_inside_conditional;

        if_conditional_context
            .cond_referenced_var_ids
            .extend(first_cond_referenced_var_ids);
        cond_referenced_var_ids = if_conditional_context.cond_referenced_var_ids.clone();

        if_conditional_context
            .assigned_var_ids
            .extend(first_cond_assigned_var_ids);
        assigned_in_conditional_var_ids = if_conditional_context.assigned_var_ids.clone();
    } else {
        cond_referenced_var_ids = first_cond_referenced_var_ids.clone();
        assigned_in_conditional_var_ids = first_cond_assigned_var_ids.clone();
    }

    let newish_var_ids = if_conditional_context
        .vars_in_scope
        .into_iter()
        .map(|(k, _)| k)
        .filter(|k| {
            !pre_condition_vars_in_scope.contains_key(k)
                && !cond_referenced_var_ids.contains(k)
                && !assigned_in_conditional_var_ids.contains_key(k)
        })
        .collect::<FxHashSet<_>>();

    if let Some(cond_type) = tast_info.get_expr_type(cond.pos()).cloned() {
        handle_paradoxical_condition(statements_analyzer, tast_info, cond.pos(), &cond_type);
    }

    cond_referenced_var_ids.retain(|k| !assigned_in_conditional_var_ids.contains_key(k));

    cond_referenced_var_ids.extend(newish_var_ids);

    let assigned_in_conditional_var_ids = FxHashMap::default();

    IfConditionalScope {
        if_body_context: if_context.unwrap(),
        post_if_context,
        outer_context: if has_outer_context_changes {
            outer_context
        } else {
            old_outer_context
        },
        cond_referenced_var_ids,
        assigned_in_conditional_var_ids,
    }
}

fn get_definitely_evaluated_expression_after_if(stmt: &aast::Expr<(), ()>) -> &aast::Expr<(), ()> {
    if let Some((bop, left, _)) = stmt.2.as_binop() {
        // todo handle <expr> === true

        if let ast::Bop::Ampamp = bop {
            return get_definitely_evaluated_expression_after_if(&left);
        }

        return stmt;
    }

    if let Some((uop, expr)) = stmt.2.as_unop() {
        if let ast::Uop::Unot = uop {
            let inner_expr = get_definitely_evaluated_expression_inside_if(&expr);

            if inner_expr != expr {
                return inner_expr;
            }
        }
    }

    stmt
}

fn get_definitely_evaluated_expression_inside_if(stmt: &aast::Expr<(), ()>) -> &aast::Expr<(), ()> {
    if let Some((bop, left, _)) = stmt.2.as_binop() {
        // todo handle <expr> === true

        if let ast::Bop::Barbar = bop {
            return get_definitely_evaluated_expression_inside_if(&left);
        }

        return stmt;
    }

    if let Some((uop, expr)) = stmt.2.as_unop() {
        if let ast::Uop::Unot = uop {
            let inner_expr = get_definitely_evaluated_expression_after_if(&expr);

            if inner_expr != expr {
                return inner_expr;
            }
        }
    }

    stmt
}

pub(crate) fn add_branch_dataflow(
    statements_analyzer: &StatementsAnalyzer,
    cond: &aast::Expr<(), ()>,
    tast_info: &mut TastInfo,
) {
    if let GraphKind::WholeProgram(_) = &tast_info.data_flow_graph.kind {
        // todo maybe useful in the future
        return;
    }

    let conditional_type = tast_info
        .expr_types
        .get(&(cond.1.start_offset(), cond.1.end_offset()));

    if let Some(conditional_type) = conditional_type {
        if !conditional_type.parent_nodes.is_empty() {
            let branch_node = DataFlowNode::get_for_variable_sink(
                "branch".to_string(),
                statements_analyzer.get_hpos(cond.pos()),
            );

            for (_, parent_node) in &conditional_type.parent_nodes {
                tast_info.data_flow_graph.add_path(
                    &parent_node,
                    &branch_node,
                    PathKind::Default,
                    None,
                    None,
                );
            }

            if tast_info.data_flow_graph.kind == GraphKind::FunctionBody {
                tast_info.data_flow_graph.add_node(branch_node);
            }
        }
    }
}

pub(crate) fn handle_paradoxical_condition(
    statements_analyzer: &StatementsAnalyzer,
    tast_info: &mut TastInfo,
    pos: &Pos,
    expr_type: &TUnion,
) {
    if expr_type.is_always_falsy() {
        tast_info.maybe_add_issue(
            Issue::new(
                IssueKind::RedundantTruthinessCheck,
                format!(
                    "Type {} is always falsy",
                    expr_type.get_id(Some(&statements_analyzer.get_codebase().interner))
                ),
                statements_analyzer.get_hpos(&pos),
            ),
            statements_analyzer.get_config(),
            statements_analyzer.get_file_path_actual(),
        );
    } else if expr_type.is_always_truthy(&statements_analyzer.get_codebase().interner) {
        tast_info.maybe_add_issue(
            Issue::new(
                IssueKind::RedundantTruthinessCheck,
                format!(
                    "Type {} is always truthy",
                    expr_type.get_id(Some(&statements_analyzer.get_codebase().interner))
                ),
                statements_analyzer.get_hpos(&pos),
            ),
            statements_analyzer.get_config(),
            statements_analyzer.get_file_path_actual(),
        );
    }
}
