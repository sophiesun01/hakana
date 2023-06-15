use crate::scope_analyzer::ScopeAnalyzer;
use crate::statements_analyzer::StatementsAnalyzer;
use crate::{config::Config, scope_context::CaseScope};
use hakana_reflection_info::analysis_result::Replacement;
use hakana_reflection_info::code_location::StmtStart;
use hakana_reflection_info::file_info::UsesFlippedMap;
use hakana_reflection_info::{
    assertion::Assertion,
    data_flow::graph::{DataFlowGraph, GraphKind, WholeProgramKind},
    functionlike_info::FunctionLikeInfo,
    issue::{get_issue_from_comment, Issue, IssueKind},
    symbol_references::SymbolReferences,
    t_union::TUnion,
};
use hakana_reflection_info::{FileSource, Interner, StrId};
use hakana_type::template::TemplateBound;
use oxidized::ast_defs;
use oxidized::tast::{Hint, Hint_};
use oxidized::{ast_defs::Pos, prim_defs::Comment};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{collections::BTreeMap, rc::Rc};

pub struct FunctionAnalysisData {
    pub expr_types: FxHashMap<(usize, usize), Rc<TUnion>>,
    pub if_true_assertions: FxHashMap<(usize, usize), FxHashMap<String, Vec<Assertion>>>,
    pub if_false_assertions: FxHashMap<(usize, usize), FxHashMap<String, Vec<Assertion>>>,
    pub data_flow_graph: DataFlowGraph,
    pub case_scopes: Vec<CaseScope>,
    pub issues_to_emit: Vec<Issue>,
    pub inferred_return_types: Vec<TUnion>,
    pub fully_matched_switch_offsets: FxHashSet<usize>,
    pub closures: FxHashMap<Pos, FunctionLikeInfo>,
    pub closure_spans: Vec<(usize, usize)>,
    pub replacements: BTreeMap<(usize, usize), Replacement>,
    pub current_stmt_offset: Option<StmtStart>,
    pub expr_fixme_positions: FxHashMap<(usize, usize), StmtStart>,
    pub symbol_references: SymbolReferences,
    pub issue_filter: Option<FxHashSet<IssueKind>>,
    pub expr_effects: FxHashMap<(usize, usize), u8>,
    pub issue_counts: FxHashMap<IssueKind, usize>,
    recording_level: usize,
    recorded_issues: Vec<Vec<Issue>>,
    hh_fixmes: BTreeMap<isize, BTreeMap<isize, Pos>>,
    pub hakana_fixme_or_ignores: BTreeMap<usize, Vec<(IssueKind, (usize, usize, u64, bool))>>,
    pub matched_ignore_positions: FxHashSet<(usize, usize)>,
    pub type_variable_bounds: FxHashMap<String, (Vec<TemplateBound>, Vec<TemplateBound>)>,
    pub first_statement_offset: Option<usize>,
}

impl FunctionAnalysisData {
    pub(crate) fn new(
        data_flow_graph: DataFlowGraph,
        file_source: &FileSource,
        comments: &Vec<&(Pos, Comment)>,
        all_custom_issues: &FxHashSet<String>,
        current_stmt_offset: Option<StmtStart>,
        hakana_fixme_or_ignores: Option<
            BTreeMap<usize, Vec<(IssueKind, (usize, usize, u64, bool))>>,
        >,
    ) -> Self {
        Self {
            expr_types: FxHashMap::default(),
            data_flow_graph,
            case_scopes: Vec::new(),
            issues_to_emit: Vec::new(),
            inferred_return_types: Vec::new(),
            fully_matched_switch_offsets: FxHashSet::default(),
            recording_level: 0,
            recorded_issues: vec![],
            closures: FxHashMap::default(),
            closure_spans: vec![],
            if_true_assertions: FxHashMap::default(),
            if_false_assertions: FxHashMap::default(),
            replacements: BTreeMap::new(),
            current_stmt_offset,
            hh_fixmes: file_source.hh_fixmes.clone(),
            symbol_references: SymbolReferences::new(),
            issue_filter: None,
            expr_effects: FxHashMap::default(),
            hakana_fixme_or_ignores: hakana_fixme_or_ignores
                .unwrap_or(get_hakana_fixmes_and_ignores(comments, all_custom_issues)),
            expr_fixme_positions: FxHashMap::default(),
            matched_ignore_positions: FxHashSet::default(),
            issue_counts: FxHashMap::default(),
            type_variable_bounds: FxHashMap::default(),
            first_statement_offset: None, 
        }
    }

    pub fn add_issue(&mut self, issue: Issue) {
        if !self.issues_to_emit.contains(&issue) {
            self.issues_to_emit.push(issue);
        }
    }

    pub fn maybe_add_issue(&mut self, mut issue: Issue, config: &Config, file_path: &str) {
        if config.ignore_mixed_issues && issue.kind.is_mixed_issue() {
            return;
        }

        if !config.allow_issue_kind_in_file(&issue.kind, file_path) {
            return;
        }

        issue.pos.insertion_start = if let Some(expr_fixme_position) = self
            .expr_fixme_positions
            .get(&(issue.pos.start_offset, issue.pos.end_offset))
        {
            Some(*expr_fixme_position)
        } else if let Some(current_stmt_offset) = self.current_stmt_offset {
            Some(current_stmt_offset)
        } else {
            None
        };

        issue.can_fix = config.add_fixmes && config.issues_to_fix.contains(&issue.kind);

        if !self.can_add_issue(&issue) {
            return;
        }

        if issue.can_fix && !issue.fixme_added {
            issue.fixme_added = self.add_issue_fixme(&issue);
        }

        self.add_issue(issue);
    }

    fn add_issue_fixme(&mut self, issue: &Issue) -> bool {
        if let Some(insertion_start) = &issue.pos.insertion_start {
            self.add_replacement(
                (insertion_start.offset, insertion_start.offset),
                Replacement::Substitute(
                    format!(
                        "/* HAKANA_FIXME[{}]{} */{}",
                        issue.kind.to_string(),
                        if let IssueKind::UnusedParameter
                        | IssueKind::UnusedAssignment
                        | IssueKind::UnusedAssignmentStatement
                        | IssueKind::UnusedStatement
                        | IssueKind::UnusedFunction
                        | IssueKind::UnusedPrivateMethod = issue.kind
                        {
                            "".to_string()
                        } else {
                            " ".to_string() + &issue.description
                        },
                        if insertion_start.add_newline {
                            "\n".to_string() + &"\t".repeat(insertion_start.column)
                        } else {
                            " ".to_string()
                        }
                    )
                    .to_string(),
                ),
            );

            true
        } else {
            false
        }
    }

    pub fn handle_hint_in_migration(
        &mut self,
        hint: &Hint,
        resolved_names: &FxHashMap<usize, StrId>,
        calling_classlike_name: &Option<StrId>,
        statements_analyzer: &StatementsAnalyzer,
    ) {
        match &*hint.1 {
            Hint_::Happly(id, type_params) => {
                let applied_type = &id.1;

                match applied_type.as_str() {
                    "int"
                    | "string"
                    | "arraykey"
                    | "bool"
                    | "float"
                    | "nonnull"
                    | "null"
                    | "nothing"
                    | "noreturn"
                    | "void"
                    | "num"
                    | "mixed"
                    | "dynamic"
                    | "vec"
                    | "HH\\vec"
                    | "HH\\varray"
                    | "varray"
                    | "dict"
                    | "HH\\dict"
                    | "HH\\darray"
                    | "darray"
                    | "classname"
                    | "typename"
                    | "vec_or_dict"
                    | "varray_or_darray"
                    | "resource"
                    | "_"
                    | "HH\\FIXME\\MISSING_RETURN_TYPE"
                    | "\\HH\\FIXME\\MISSING_RETURN_TYPE" => {}
                    _ => {
                        if let Some(resolved_name) = resolved_names.get(&id.0.start_offset()) {
                            self.handle_classlike_reference_in_migration(
                                resolved_name,
                                (id.0.start_offset(), id.0.end_offset()),
                                calling_classlike_name,
                                statements_analyzer,
                            );
                        }
                    }
                }

                for type_param in type_params {
                    self.handle_hint_in_migration(
                        type_param,
                        resolved_names,
                        calling_classlike_name,
                        statements_analyzer,
                    );
                }
            }
            Hint_::Hshape(shape_info) => {
                for field in &shape_info.field_map {
                    self.handle_hint_in_migration(
                        &field.hint,
                        resolved_names,
                        calling_classlike_name,
                        statements_analyzer,
                    );

                    match &field.name {
                        ast_defs::ShapeFieldName::SFclassConst(lhs, _) => {
                            let lhs_name = resolved_names.get(&lhs.0.start_offset()).unwrap();
                            self.handle_classlike_reference_in_migration(
                                lhs_name,
                                (lhs.0.start_offset(), lhs.0.end_offset()),
                                calling_classlike_name,
                                statements_analyzer,
                            );
                        }
                        _ => {}
                    }
                }
            }
            Hint_::Htuple(tuple_hints) => {
                for hint in tuple_hints {
                    self.handle_hint_in_migration(
                        hint,
                        resolved_names,
                        calling_classlike_name,
                        statements_analyzer,
                    );
                }
            }
            Hint_::Hoption(inner) => {
                self.handle_hint_in_migration(
                    inner,
                    resolved_names,
                    calling_classlike_name,
                    statements_analyzer,
                );
            }
            Hint_::Hfun(hint_fun) => {
                for param_hint in &hint_fun.param_tys {
                    self.handle_hint_in_migration(
                        param_hint,
                        resolved_names,
                        calling_classlike_name,
                        statements_analyzer,
                    );
                }
                self.handle_hint_in_migration(
                    &hint_fun.return_ty,
                    resolved_names,
                    calling_classlike_name,
                    statements_analyzer,
                );
            }
            Hint_::Haccess(class, _) => {
                self.handle_hint_in_migration(
                    class,
                    resolved_names,
                    calling_classlike_name,
                    statements_analyzer,
                );
            }
            Hint_::Hsoft(hint) => {
                self.handle_hint_in_migration(
                    hint,
                    resolved_names,
                    calling_classlike_name,
                    statements_analyzer,
                );
            }
            _ => {}
        }
    }

    pub fn handle_classlike_reference_in_migration(
        &mut self,
        classlike_name: &StrId,
        range: (usize, usize),
        calling_classlike_name: &Option<StrId>,
        statements_analyzer: &StatementsAnalyzer,
    ) {
        let config = statements_analyzer.get_config();
        let interner = statements_analyzer.get_interner();
        let codebase = statements_analyzer.get_codebase();

        let classlike_name_str = interner.lookup(classlike_name);

        // if we're outside a moved class, but we're changing all references to a class
        if let Some(classlikes_to_rename) = &config.classlikes_to_rename {
            if let Some(destination_name_str) = classlikes_to_rename.get(classlike_name_str) {
                let uses_flipped_maps = &codebase
                    .files
                    .get(statements_analyzer.get_file_path())
                    .unwrap()
                    .uses_flipped_map;

                let mut source_namespace = statements_analyzer.get_namespace().clone();

                if let Some(calling_classlike_name) = calling_classlike_name {
                    let calling_classlike_name_str = interner.lookup(calling_classlike_name);

                    if let Some(destination_calling_name_str) =
                        classlikes_to_rename.get(calling_classlike_name_str)
                    {
                        let mut new_source_parts = destination_calling_name_str
                            .split("\\")
                            .into_iter()
                            .collect::<Vec<_>>();
                        new_source_parts.pop();

                        if new_source_parts.is_empty() {
                            source_namespace = None;
                        } else {
                            source_namespace = Some(new_source_parts.join("\\"));
                        }
                    }
                }

                let class_name = FunctionAnalysisData::get_class_name_from_uses(
                    destination_name_str.to_string(),
                    source_namespace,
                    uses_flipped_maps,
                    interner,
                );

                self.replacements
                    .insert(range, Replacement::Substitute(class_name));
            }
        }
    }

    pub(crate) fn get_class_name_from_uses(
        value: String,
        namespace: Option<String>,
        uses_flipped_map: &UsesFlippedMap,
        interner: &Interner,
    ) -> String {
        if let Some(namespace) = &namespace {
            if value.starts_with(&(namespace.clone() + "\\")) {
                let candidate = &value[namespace.len() + 1..];
                let candidate_parts = candidate.split("\\");
                let base_namespace = interner
                    .get(candidate_parts.into_iter().next().unwrap())
                    .unwrap();

                if !uses_flipped_map
                    .type_aliases_flipped
                    .contains_key(&base_namespace)
                {
                    return candidate.to_string();
                }
            }
        } else {
            if !value.contains("\\") {
                return value;
            }
        }

        // check if any of the "use namespace ..." statements are a match
        if value.contains("\\") {
            let mut parts = value.split("\\").into_iter().collect::<Vec<_>>();
            let mut suffix = parts.pop().unwrap().to_string();

            while !parts.is_empty() {
                let base_namespace = interner.get(&parts.join("\\")).unwrap();

                if let Some(namespace_id) = uses_flipped_map
                    .namespace_aliases_flipped
                    .get(&base_namespace)
                {
                    return interner.lookup(namespace_id).to_string() + "\\" + &suffix;
                }

                suffix = parts.pop().unwrap().to_string() + "\\" + &suffix;
            }
        }

        if let Some(_) = &namespace {
            "\\".to_string() + &value
        } else {
            value
        }
    }

    pub fn can_add_issue(&mut self, issue: &Issue) -> bool {
        if matches!(
            &self.data_flow_graph.kind,
            GraphKind::WholeProgram(WholeProgramKind::Taint)
        ) {
            return matches!(issue.kind, IssueKind::TaintedData(_));
        }

        if self.covered_by_hh_fixme(&issue.kind, issue.pos.start_line)
            || self.covered_by_hh_fixme(&issue.kind, issue.pos.start_line - 1)
        {
            return false;
        }

        for hakana_fixme_or_ignores in &self.hakana_fixme_or_ignores {
            if hakana_fixme_or_ignores.0 == &issue.pos.start_line
                || hakana_fixme_or_ignores.0 == &(issue.pos.start_line - 1)
                || hakana_fixme_or_ignores.0 == &(issue.pos.end_line - 1)
            {
                for line_issue in hakana_fixme_or_ignores.1 {
                    if line_issue.0 == issue.kind
                        || (line_issue.0 == IssueKind::UnusedAssignment
                            && issue.kind == IssueKind::UnusedAssignmentStatement)
                    {
                        self.matched_ignore_positions
                            .insert((line_issue.1 .0, line_issue.1 .1));

                        if self.recorded_issues.is_empty() {
                            *self.issue_counts.entry(issue.kind.clone()).or_insert(0) += 1;
                        }
                        return false;
                    }
                }
            }
        }

        if let Some(recorded_issues) = self.recorded_issues.last_mut() {
            recorded_issues.push(issue.clone());
            return false;
        }

        *self.issue_counts.entry(issue.kind.clone()).or_insert(0) += 1;

        if let Some(issue_filter) = &self.issue_filter {
            if !issue_filter.contains(&issue.kind) {
                return false;
            }
        }

        return true;
    }

    fn covered_by_hh_fixme(&mut self, issue_kind: &IssueKind, start_line: usize) -> bool {
        if let Some(fixmes) = self.hh_fixmes.get(&(start_line as isize)) {
            for (hack_error, _) in fixmes {
                match *hack_error {
                    // Unify error
                    4110 => match &issue_kind {
                        IssueKind::FalsableReturnStatement
                        | IssueKind::FalseArgument
                        | IssueKind::ImpossibleAssignment
                        | IssueKind::InvalidArgument
                        | IssueKind::InvalidReturnStatement
                        | IssueKind::InvalidReturnType
                        | IssueKind::InvalidReturnValue
                        | IssueKind::LessSpecificArgument
                        | IssueKind::LessSpecificNestedArgumentType
                        | IssueKind::LessSpecificNestedReturnStatement
                        | IssueKind::LessSpecificReturnStatement
                        | IssueKind::MixedArgument
                        | IssueKind::MixedArrayAccess
                        | IssueKind::MixedArrayAssignment
                        | IssueKind::MixedMethodCall
                        | IssueKind::MixedReturnStatement
                        | IssueKind::MixedPropertyAssignment
                        | IssueKind::MixedPropertyTypeCoercion
                        | IssueKind::PropertyTypeCoercion
                        | IssueKind::NonNullableReturnType
                        | IssueKind::NullArgument
                        | IssueKind::NullablePropertyAssignment
                        | IssueKind::NullableReturnStatement
                        | IssueKind::NullableReturnValue
                        | IssueKind::PossiblyFalseArgument
                        | IssueKind::PossiblyInvalidArgument
                        | IssueKind::PossiblyNullArgument
                        | IssueKind::InvalidPropertyAssignmentValue
                        | IssueKind::LessSpecificNestedAnyReturnStatement
                        | IssueKind::LessSpecificNestedAnyArgumentType => {
                            return true;
                        }
                        _ => {}
                    },
                    // type inference failed
                    4297 => match &issue_kind {
                        IssueKind::MixedAnyArgument
                        | IssueKind::MixedAnyArrayAccess
                        | IssueKind::MixedAnyArrayAssignment
                        | IssueKind::MixedAnyArrayOffset
                        | IssueKind::MixedAnyAssignment
                        | IssueKind::MixedAnyMethodCall
                        | IssueKind::MixedAnyPropertyAssignment
                        | IssueKind::MixedAnyPropertyTypeCoercion
                        | IssueKind::MixedAnyReturnStatement
                        | IssueKind::MixedArgument
                        | IssueKind::MixedArrayAccess
                        | IssueKind::MixedArrayAssignment
                        | IssueKind::MixedArrayOffset
                        | IssueKind::MixedMethodCall
                        | IssueKind::MixedPropertyAssignment
                        | IssueKind::MixedPropertyTypeCoercion
                        | IssueKind::MixedReturnStatement => {
                            return true;
                        }
                        _ => {}
                    },
                    // RequiredFieldIsOptional
                    4163 => match &issue_kind {
                        IssueKind::InvalidArgument
                        | IssueKind::InvalidReturnStatement
                        | IssueKind::InvalidReturnType
                        | IssueKind::InvalidReturnValue
                        | IssueKind::LessSpecificArgument
                        | IssueKind::LessSpecificNestedArgumentType
                        | IssueKind::LessSpecificNestedReturnStatement
                        | IssueKind::LessSpecificReturnStatement
                        | IssueKind::PropertyTypeCoercion
                        | IssueKind::PossiblyInvalidArgument => {
                            return true;
                        }
                        _ => {}
                    },
                    4323 => match &issue_kind {
                        IssueKind::PossiblyNullArgument => {
                            return true;
                        }
                        _ => {}
                    },
                    4063 => match &issue_kind {
                        IssueKind::MixedArrayAccess | IssueKind::PossiblyNullArrayAccess => {
                            return true;
                        }
                        _ => {}
                    },
                    4064 => match &issue_kind {
                        IssueKind::PossiblyNullArgument | IssueKind::PossiblyNullPropertyFetch => {
                            return true;
                        }
                        _ => {}
                    },
                    4005 => match &issue_kind {
                        IssueKind::MixedArrayAccess => {
                            return true;
                        }
                        _ => {}
                    },
                    2049 => match &issue_kind {
                        IssueKind::NonExistentMethod => return true,
                        IssueKind::NonExistentClass => return true,
                        _ => {}
                    },
                    // missing member
                    4053 => match &issue_kind {
                        IssueKind::NonExistentMethod | IssueKind::NonExistentXhpAttribute => {
                            return true
                        }
                        _ => {}
                    },
                    // missing shape field or shape field unknown
                    4057 | 4138 => match &issue_kind {
                        IssueKind::LessSpecificArgument
                        | IssueKind::LessSpecificReturnStatement
                        | IssueKind::InvalidReturnStatement => return true,
                        _ => {}
                    },
                    4062 => match &issue_kind {
                        IssueKind::MixedMethodCall => return true,
                        _ => {}
                    },
                    4321 | 4108 => match &issue_kind {
                        IssueKind::UndefinedStringArrayOffset
                        | IssueKind::UndefinedIntArrayOffset
                        | IssueKind::ImpossibleNonnullEntryCheck => return true,
                        _ => {}
                    },
                    4165 => match &issue_kind {
                        IssueKind::PossiblyUndefinedStringArrayOffset
                        | IssueKind::PossiblyUndefinedIntArrayOffset => return true,
                        _ => {}
                    },
                    4249 | 4250 => match &issue_kind {
                        IssueKind::RedundantKeyCheck | IssueKind::ImpossibleKeyCheck => {
                            return true
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        false
    }

    pub fn start_recording_issues(&mut self) {
        self.recording_level += 1;
        self.recorded_issues.push(vec![]);
    }

    pub fn stop_recording_issues(&mut self) {
        self.recording_level -= 1;
        self.recorded_issues.pop();
    }

    pub fn clear_currently_recorded_issues(&mut self) -> Vec<Issue> {
        let issues = self.recorded_issues.pop().unwrap();
        self.recorded_issues.push(vec![]);
        issues
    }

    pub fn bubble_up_issue(&mut self, issue: Issue) {
        if self.recording_level == 0 {
            if issue.can_fix {
                self.add_issue_fixme(&issue);
            }

            *self.issue_counts.entry(issue.kind.clone()).or_insert(0) += 1;

            if let Some(issue_filter) = &self.issue_filter {
                if !issue_filter.contains(&issue.kind) {
                    return;
                }
            }

            self.add_issue(issue);
            return;
        }

        if let Some(issues) = self.recorded_issues.last_mut() {
            issues.push(issue);
        }
    }

    pub(crate) fn copy_effects(&mut self, source_pos_1: &Pos, destination_pos: &Pos) {
        self.expr_effects.insert(
            (destination_pos.start_offset(), destination_pos.end_offset()),
            *self
                .expr_effects
                .get(&(source_pos_1.start_offset(), source_pos_1.end_offset()))
                .unwrap_or(&0),
        );
    }

    pub(crate) fn combine_effects(
        &mut self,
        source_pos_1: &Pos,
        source_pos_2: &Pos,
        destination_pos: &Pos,
    ) {
        self.expr_effects.insert(
            (destination_pos.start_offset(), destination_pos.end_offset()),
            self.expr_effects
                .get(&(source_pos_1.start_offset(), source_pos_1.end_offset()))
                .unwrap_or(&0)
                | self
                    .expr_effects
                    .get(&(source_pos_2.start_offset(), source_pos_2.end_offset()))
                    .unwrap_or(&0),
        );
    }

    pub(crate) fn combine_effects_with(
        &mut self,
        source_pos_1: &Pos,
        source_pos_2: &Pos,
        destination_pos: &Pos,
        effect: u8,
    ) {
        self.expr_effects.insert(
            (destination_pos.start_offset(), destination_pos.end_offset()),
            self.expr_effects
                .get(&(source_pos_1.start_offset(), source_pos_1.end_offset()))
                .unwrap_or(&0)
                | self
                    .expr_effects
                    .get(&(source_pos_2.start_offset(), source_pos_2.end_offset()))
                    .unwrap_or(&0)
                | effect,
        );
    }

    pub(crate) fn is_pure(&self, source_pos: &Pos) -> bool {
        if let Some(expr_effect) = self
            .expr_effects
            .get(&(source_pos.start_offset(), source_pos.end_offset()))
        {
            expr_effect == &0
        } else {
            true
        }
    }

    #[inline]
    pub fn set_expr_type(&mut self, pos: &Pos, t: TUnion) {
        self.expr_types
            .insert((pos.start_offset(), pos.end_offset()), Rc::new(t));
    }

    #[inline]
    pub fn get_expr_type(&self, pos: &Pos) -> Option<&TUnion> {
        if let Some(t) = self.expr_types.get(&(pos.start_offset(), pos.end_offset())) {
            Some(&**t)
        } else {
            None
        }
    }

    #[inline]
    pub fn set_rc_expr_type(&mut self, pos: &Pos, t: Rc<TUnion>) {
        self.expr_types
            .insert((pos.start_offset(), pos.end_offset()), t);
    }

    #[inline]
    pub fn get_rc_expr_type(&self, pos: &Pos) -> Option<&Rc<TUnion>> {
        if let Some(t) = self.expr_types.get(&(pos.start_offset(), pos.end_offset())) {
            Some(t)
        } else {
            None
        }
    }

    pub(crate) fn get_unused_hakana_fixme_positions(&self) -> Vec<(usize, usize, u64, bool)> {
        let mut unused_fixme_positions = vec![];

        for hakana_fixme_or_ignores in &self.hakana_fixme_or_ignores {
            for line_issue in hakana_fixme_or_ignores.1 {
                if !self
                    .matched_ignore_positions
                    .contains(&(line_issue.1 .0, line_issue.1 .1))
                {
                    unused_fixme_positions.push(line_issue.1);
                }
            }
        }

        unused_fixme_positions
    }

    pub fn add_replacement(&mut self, offsets: (usize, usize), replacement: Replacement) -> bool {
        for ((start, end), _) in &self.replacements {
            if (offsets.0 >= *start && offsets.0 <= *end)
                || (offsets.1 >= *start && offsets.1 <= *end)
            {
                return false;
            }

            if (*start >= offsets.0 && *start <= offsets.1)
                || (*end >= offsets.0 && *end <= offsets.1)
            {
                return false;
            }
        }

        self.replacements.insert(offsets, replacement);
        true
    }
}

fn get_hakana_fixmes_and_ignores(
    comments: &Vec<&(Pos, Comment)>,
    all_custom_issues: &FxHashSet<String>,
) -> BTreeMap<usize, Vec<(IssueKind, (usize, usize, u64, bool))>> {
    let mut hakana_fixme_or_ignores = BTreeMap::new();
    for (pos, comment) in comments {
        match comment {
            Comment::CmtBlock(text) => {
                let trimmed_text = if text.starts_with("*") {
                    text[1..].trim()
                } else {
                    text.trim()
                };

                if let Some(Ok(issue_kind)) =
                    get_issue_from_comment(trimmed_text, all_custom_issues)
                {
                    hakana_fixme_or_ignores
                        .entry(pos.line())
                        .or_insert_with(Vec::new)
                        .push((
                            issue_kind,
                            (
                                pos.start_offset(),
                                pos.end_offset(),
                                pos.to_raw_span().start.beg_of_line(),
                                false,
                            ),
                        ));
                }
            }
            _ => {}
        }
    }
    hakana_fixme_or_ignores
}
