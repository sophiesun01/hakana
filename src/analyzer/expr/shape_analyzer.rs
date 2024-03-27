use crate::{
    expression_analyzer, function_analysis_data::FunctionAnalysisData,
    scope_analyzer::ScopeAnalyzer, scope_context::ScopeContext,
    statements_analyzer::StatementsAnalyzer, stmt_analyzer::AnalysisError,
};
use hakana_reflection_info::{
    code_location::StmtStart,
    data_flow::{
        graph::{GraphKind, WholeProgramKind},
        node::DataFlowNode,
        path::{ArrayDataKind, PathKind},
    },
    t_atomic::{DictKey, TAtomic},
    t_union::TUnion,
};
use hakana_type::{get_mixed_any, wrap_atomic};
use oxidized::{
    aast,
    ast_defs::{Pos, ShapeFieldName},
};
use rustc_hash::FxHashSet;
use std::{collections::BTreeMap, sync::Arc};

pub(crate) fn analyze(
    statements_analyzer: &StatementsAnalyzer,
    shape_fields: &Vec<(ShapeFieldName, aast::Expr<(), ()>)>,
    pos: &Pos,
    analysis_data: &mut FunctionAnalysisData,
    context: &mut ScopeContext,
) -> Result<(), AnalysisError> {
    let codebase = statements_analyzer.get_codebase();

    let mut parent_nodes = vec![];

    let mut effects = 0;

    let mut known_items = BTreeMap::new();
    for (name, value_expr) in shape_fields {
        let start_pos = match name {
            ShapeFieldName::SFlitInt(name) => &name.0,
            ShapeFieldName::SFlitStr(name) => &name.0,
            ShapeFieldName::SFclassConst(lhs, _) => &lhs.0,
        };

        if let Some(ref mut current_stmt_offset) = analysis_data.current_stmt_offset {
            if current_stmt_offset.line != start_pos.line() as u32 {
                *current_stmt_offset = StmtStart {
                    offset: start_pos.start_offset() as u32,
                    line: start_pos.line() as u32,
                    column: start_pos.to_raw_span().start.column() as u16,
                    add_newline: true,
                };
            }
        }

        let name = match name {
            ShapeFieldName::SFlitInt(name) => Some(DictKey::Int(name.1.parse::<u64>().unwrap())),
            ShapeFieldName::SFlitStr(name) => Some(DictKey::String(name.1.to_string())),
            ShapeFieldName::SFclassConst(lhs, name) => {
                let lhs_name = if let Some(name) = statements_analyzer
                    .get_file_analyzer()
                    .resolved_names
                    .get(&(lhs.0.start_offset() as u32))
                {
                    name
                } else {
                    return Err(AnalysisError::InternalError(
                        format!("unknown classname at pos {}", &lhs.1),
                        statements_analyzer.get_hpos(&lhs.0),
                    ));
                };

                let constant_type = codebase.get_class_constant_type(
                    lhs_name,
                    false,
                    &statements_analyzer.get_interner().get(&name.1).unwrap(),
                    FxHashSet::default(),
                );

                if let Some(constant_type) = constant_type {
                    if constant_type.is_single() {
                        let single = constant_type.get_single_owned();

                        match single {
                            TAtomic::TEnumLiteralCase {
                                enum_name,
                                member_name,
                                ..
                            } => Some(DictKey::Enum(enum_name, member_name)),
                            TAtomic::TLiteralString { value } => Some(DictKey::String(value)),
                            _ => None,
                        }
                    } else {
                        println!(
                            "surprising union type {}",
                            constant_type.get_id(Some(statements_analyzer.get_interner()))
                        );
                        panic!();
                    }
                } else {
                    return Err(AnalysisError::InternalError(
                        format!(
                            "unknown constant {}::{}",
                            statements_analyzer.get_interner().lookup(lhs_name),
                            &name.1
                        ),
                        statements_analyzer.get_hpos(&name.0),
                    ));
                }
            }
        };

        // Now check types of the values
        expression_analyzer::analyze(
            statements_analyzer,
            value_expr,
            analysis_data,
            context,
            &mut None,
        )?;

        effects |= analysis_data
            .expr_effects
            .get(&(
                value_expr.pos().start_offset() as u32,
                value_expr.pos().end_offset() as u32,
            ))
            .unwrap_or(&0);

        if let Some(name) = name {
            let value_item_type = analysis_data
                .get_expr_type(value_expr.pos())
                .cloned()
                .unwrap_or(get_mixed_any());

            if let Some(new_parent_node) = add_shape_value_dataflow(
                statements_analyzer,
                &value_item_type,
                analysis_data,
                &match &name {
                    DictKey::Int(i) => i.to_string(),
                    DictKey::String(k) => k.clone(),
                    DictKey::Enum(class_name, member_name) => {
                        statements_analyzer
                            .get_interner()
                            .lookup(class_name)
                            .to_string()
                            + "::"
                            + statements_analyzer.get_interner().lookup(member_name)
                    }
                },
                value_expr,
            ) {
                parent_nodes.push(new_parent_node);
            }

            known_items.insert(name, (false, Arc::new(value_item_type)));
        }
    }

    analysis_data.expr_effects.insert(
        (pos.start_offset() as u32, pos.end_offset() as u32),
        effects,
    );

    let mut new_dict = wrap_atomic(TAtomic::TDict {
        known_items: if !known_items.is_empty() {
            Some(known_items)
        } else {
            None
        },
        params: None,
        non_empty: true,
        shape_name: None,
    });

    new_dict.parent_nodes = parent_nodes;

    analysis_data.set_expr_type(pos, new_dict);

    Ok(())
}

fn add_shape_value_dataflow(
    statements_analyzer: &StatementsAnalyzer,
    value_type: &TUnion,
    analysis_data: &mut FunctionAnalysisData,
    key_value: &String,
    value: &aast::Expr<(), ()>,
) -> Option<DataFlowNode> {
    if value_type.parent_nodes.is_empty()
        || (matches!(
            &analysis_data.data_flow_graph.kind,
            GraphKind::WholeProgram(WholeProgramKind::Taint)
        ) && !value_type.has_taintable_value())
    {
        return None;
    }

    let node_name = format!("array[{}]", key_value);

    let new_parent_node = DataFlowNode::get_for_array_item(
        node_name,
        statements_analyzer.get_hpos(value.pos()),
        !value_type.parent_nodes.is_empty(),
    );
    analysis_data
        .data_flow_graph
        .add_node(new_parent_node.clone());

    // TODO add taint event dispatches

    for parent_node in value_type.parent_nodes.iter() {
        analysis_data.data_flow_graph.add_path(
            parent_node,
            &new_parent_node,
            PathKind::ArrayAssignment(ArrayDataKind::ArrayValue, key_value.clone()),
            vec![],
            vec![],
        );
    }

    Some(new_parent_node)
}
