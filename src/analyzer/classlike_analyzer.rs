use std::collections::BTreeMap;

use crate::expression_analyzer;
use crate::file_analyzer::FileAnalyzer;
use crate::function_analysis_data::FunctionAnalysisData;
use crate::functionlike_analyzer::FunctionLikeAnalyzer;
use crate::scope_analyzer::ScopeAnalyzer;
use crate::scope_context::ScopeContext;
use crate::statements_analyzer::StatementsAnalyzer;
use hakana_reflection_info::analysis_result::{AnalysisResult, Replacement};
use hakana_reflection_info::codebase_info::symbols::SymbolKind;
use hakana_reflection_info::data_flow::graph::{DataFlowGraph, GraphKind};
use hakana_reflection_info::function_context::FunctionContext;
use oxidized::aast;

pub(crate) struct ClassLikeAnalyzer<'a> {
    file_analyzer: &'a FileAnalyzer<'a>,
}

impl<'a> ClassLikeAnalyzer<'a> {
    pub fn new(file_analyzer: &'a FileAnalyzer) -> Self {
        Self { file_analyzer }
    }

    pub fn analyze(
        &mut self,
        stmt: &aast::Class_<(), ()>,
        statements_analyzer: &StatementsAnalyzer,
        analysis_result: &mut AnalysisResult,
    ) {
        let resolved_names = self.file_analyzer.resolved_names.clone();
        let name = resolved_names
            .get(&stmt.name.0.start_offset())
            .unwrap()
            .clone();

        let codebase = self.file_analyzer.get_codebase();

        if self.file_analyzer.analysis_config.ast_diff {
            if self.file_analyzer.codebase.safe_symbols.contains(&name) {
                return;
            }
        }

        let classlike_storage = codebase.classlike_infos.get(&name).unwrap();

        let config = statements_analyzer.get_config();

        let mut analysis_data = FunctionAnalysisData::new(
            DataFlowGraph::new(GraphKind::FunctionBody),
            statements_analyzer.get_file_analyzer().get_file_source(),
            &statements_analyzer.comments,
            &config.all_custom_issues,
            None,
            None,
        );

        for parent_class in &classlike_storage.all_parent_classes {
            analysis_result
                .symbol_references
                .add_symbol_reference_to_symbol(name.clone(), parent_class.clone(), true);
        }

        for parent_interface in &classlike_storage.all_parent_interfaces {
            analysis_result
                .symbol_references
                .add_symbol_reference_to_symbol(name.clone(), parent_interface.clone(), true);
        }

        if let Some(_) = statements_analyzer.get_config().classlikes_to_rename {
            for parent_class_hint in &stmt.extends {
                analysis_data.handle_hint_in_migration(
                    parent_class_hint,
                    &resolved_names,
                    &Some(name),
                    statements_analyzer,
                );
            }

            for implements_hint in &stmt.implements {
                analysis_data.handle_hint_in_migration(
                    implements_hint,
                    &resolved_names,
                    &Some(name),
                    statements_analyzer,
                );
            }

            for use_hint in &stmt.uses {
                analysis_data.handle_hint_in_migration(
                    use_hint,
                    &resolved_names,
                    &Some(name),
                    statements_analyzer,
                );
            }
        }

        for trait_name in &classlike_storage.used_traits {
            analysis_result
                .symbol_references
                .add_symbol_reference_to_symbol(name.clone(), trait_name.clone(), true);
        }

        if let Some(classes_to_rename) = &config.classlikes_to_rename {
            let class_name_str = statements_analyzer.get_interner().lookup(&name);
            if let Some(destination_classlike_name) = classes_to_rename.get(class_name_str) {
                let mut source_class_parts = class_name_str.split("\\").collect::<Vec<_>>();
                let mut destination_class_parts =
                    destination_classlike_name.split("\\").collect::<Vec<_>>();

                let source_last_name = source_class_parts.pop().unwrap();
                let destination_last_name = destination_class_parts.pop().unwrap();

                let source_ns = source_class_parts.join("\\");
                let destination_ns = destination_class_parts.join("\\");

                if source_ns != destination_ns {
                    if let Some(namespace_location) = classlike_storage.namespace_bounds {
                        analysis_result
                            .replacements
                            .entry(*statements_analyzer.get_file_path())
                            .or_insert_with(BTreeMap::new)
                            .insert(namespace_location, Replacement::Substitute(destination_ns));
                    }
                }

                if source_last_name != destination_last_name {
                    analysis_result
                        .replacements
                        .entry(*statements_analyzer.get_file_path())
                        .or_insert_with(BTreeMap::new)
                        .insert(
                            (stmt.name.0.start_offset(), stmt.name.0.end_offset()),
                            Replacement::Substitute(destination_last_name.to_string()),
                        );
                }
            }
        }

        let mut function_context = FunctionContext::new();
        function_context.calling_class = Some(name.clone());

        let mut class_context = ScopeContext::new(function_context);

        for constant in &stmt.consts {
            match &constant.kind {
                aast::ClassConstKind::CCAbstract(Some(expr))
                | aast::ClassConstKind::CCConcrete(expr) => {
                    expression_analyzer::analyze(
                        statements_analyzer,
                        expr,
                        &mut analysis_data,
                        &mut class_context,
                        &mut None,
                    );
                }
                _ => {}
            }
        }

        for var in &stmt.vars {
            if let Some(default) = &var.expr {
                expression_analyzer::analyze(
                    statements_analyzer,
                    default,
                    &mut analysis_data,
                    &mut class_context,
                    &mut None,
                );
            }
        }

        analysis_result
            .symbol_references
            .extend(analysis_data.symbol_references);

        for method in &stmt.methods {
            if method.abstract_ || matches!(classlike_storage.kind, SymbolKind::Interface) {
                continue;
            }

            let mut method_analyzer = FunctionLikeAnalyzer::new(self.file_analyzer);
            method_analyzer.analyze_method(method, classlike_storage, analysis_result);
        }
    }
}
