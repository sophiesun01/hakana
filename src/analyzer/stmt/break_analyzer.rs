use std::rc::Rc;

use super::control_analyzer::BreakContext;
use crate::{
    scope_analyzer::ScopeAnalyzer,
    scope_context::{control_action::ControlAction, loop_scope::LoopScope, ScopeContext},
};
use crate::{statements_analyzer::StatementsAnalyzer, typed_ast::FunctionAnalysisData};
use hakana_type::{combine_optional_union_types, combine_union_types};
use rustc_hash::FxHashMap;

pub(crate) fn analyze(
    statements_analyzer: &StatementsAnalyzer,
    analysis_data: &mut FunctionAnalysisData,
    context: &mut ScopeContext,
    loop_scope: &mut Option<LoopScope>,
) {
    let mut leaving_switch = true;

    let codebase = statements_analyzer.get_codebase();

    if let Some(loop_scope) = loop_scope {
        if if let Some(last_break_type) = context.break_types.last() {
            last_break_type == &BreakContext::Switch
        } else {
            false
        } {
            loop_scope.final_actions.push(ControlAction::LeaveSwitch);
        } else {
            leaving_switch = false;
            loop_scope.final_actions.push(ControlAction::Break);
        }

        for (var_id, var_type) in &context.vars_in_scope {
            loop_scope.possibly_redefined_loop_parent_vars.insert(
                var_id.clone(),
                if let Some(existing_redefined_loop_parent_var) =
                    loop_scope.possibly_redefined_loop_parent_vars.get(var_id)
                {
                    Rc::new(hakana_type::add_union_type(
                        (**var_type).clone(),
                        existing_redefined_loop_parent_var,
                        codebase,
                        false,
                    ))
                } else {
                    var_type.clone()
                },
            );
        }

        if loop_scope.iteration_count == 0 {
            for (var_id, var_type) in &context.vars_in_scope {
                if !loop_scope.parent_context_vars.contains_key(var_id) {
                    loop_scope.possibly_defined_loop_parent_vars.insert(
                        var_id.clone(),
                        combine_optional_union_types(
                            Some(var_type),
                            loop_scope.possibly_defined_loop_parent_vars.get(var_id),
                            codebase,
                        ),
                    );
                }
            }
        }

        if let Some(finally_scope) = context.finally_scope.clone() {
            let mut finally_scope = (*finally_scope).borrow_mut();
            for (var_id, var_type) in &context.vars_in_scope {
                if let Some(finally_type) = finally_scope.vars_in_scope.get_mut(var_id) {
                    *finally_type = Rc::new(combine_union_types(
                        &finally_type,
                        var_type,
                        codebase,
                        false,
                    ));
                } else {
                    finally_scope
                        .vars_in_scope
                        .insert(var_id.clone(), var_type.clone());
                }
            }
        }
    }

    let case_scope = analysis_data.case_scopes.last_mut();

    if let Some(case_scope) = case_scope {
        if leaving_switch {
            let mut new_break_vars = case_scope
                .break_vars
                .clone()
                .unwrap_or(FxHashMap::default());

            for (var_id, var_type) in &context.vars_in_scope {
                new_break_vars.insert(
                    var_id.clone(),
                    combine_optional_union_types(
                        Some(var_type),
                        new_break_vars.get(var_id),
                        codebase,
                    ),
                );
            }

            case_scope.break_vars = Some(new_break_vars);
        }
    }

    context.has_returned = true;
}
