use ndarray::{Array1, Array2};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

use crate::{
    codebase_info::CodebaseInfo,
    diff::CodebaseDiff,
    function_context::{FunctionContext, FunctionLikeIdentifier},
    StrId,
};

pub enum ReferenceSource {
    Symbol(bool, StrId),
    ClasslikeMember(bool, StrId, StrId),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SymbolReferences {
    // A lookup table of all symbols (classes, functions, enums etc) that reference a classlike member
    // (class method, enum case, class property etc)
    pub symbol_references_to_members: FxHashMap<StrId, FxHashSet<(StrId, StrId)>>,

    // A lookup table of all symbols (classes, functions, enums etc) that reference a classlike member
    // (class method, enum case, class property etc) from their signature
    pub symbol_references_to_members_in_signature: FxHashMap<StrId, FxHashSet<(StrId, StrId)>>,

    // A lookup table of all symbols (classes, functions, enums etc) that reference another symbol
    pub symbol_references_to_symbols: FxHashMap<StrId, FxHashSet<StrId>>,

    // A lookup table of all symbols (classes, functions, enums etc) that reference another symbol
    // from that symbol's signature (e.g. a function return type, or class implements)
    pub symbol_references_to_symbols_in_signature: FxHashMap<StrId, FxHashSet<StrId>>,

    // A lookup table of all classlike members that reference another classlike member
    pub classlike_member_references_to_members:
        FxHashMap<(StrId, StrId), FxHashSet<(StrId, StrId)>>,

    // A lookup table of all classlike members that reference another classlike member
    pub classlike_member_references_to_members_in_signature:
        FxHashMap<(StrId, StrId), FxHashSet<(StrId, StrId)>>,

    // A lookup table of all classlike members that reference another symbol
    pub classlike_member_references_to_symbols: FxHashMap<(StrId, StrId), FxHashSet<StrId>>,

    // A lookup table of all classlike members that reference another symbol
    pub classlike_member_references_to_symbols_in_signature:
        FxHashMap<(StrId, StrId), FxHashSet<StrId>>,

    // A lookup table of all symbols (classes, functions, enums etc) that reference a classlike member
    // (class method, enum case, class property etc)
    pub symbol_references_to_overridden_members: FxHashMap<StrId, FxHashSet<(StrId, StrId)>>,

    // A lookup table of all classlike members that reference another classlike member
    pub classlike_member_references_to_overridden_members:
        FxHashMap<(StrId, StrId), FxHashSet<(StrId, StrId)>>,

    // A lookup table used for getting all the functions that reference a method's return value
    // This is used for dead code detection when we want to see what return values are unused
    pub functionlike_references_to_functionlike_returns:
        FxHashMap<FunctionLikeIdentifier, FxHashSet<FunctionLikeIdentifier>>,
}

impl SymbolReferences {
    pub fn new() -> Self {
        Self {
            symbol_references_to_members: FxHashMap::default(),
            symbol_references_to_symbols: FxHashMap::default(),
            classlike_member_references_to_members: FxHashMap::default(),
            classlike_member_references_to_symbols: FxHashMap::default(),
            functionlike_references_to_functionlike_returns: FxHashMap::default(),
            symbol_references_to_overridden_members: FxHashMap::default(),
            classlike_member_references_to_overridden_members: FxHashMap::default(),
            symbol_references_to_members_in_signature: FxHashMap::default(),
            symbol_references_to_symbols_in_signature: FxHashMap::default(),
            classlike_member_references_to_members_in_signature: FxHashMap::default(),
            classlike_member_references_to_symbols_in_signature: FxHashMap::default(),
        }
    }

    pub fn add_symbol_reference_to_class_member(
        &mut self,
        referencing_symbol: StrId,
        class_member: (StrId, StrId),
        in_signature: bool,
    ) {
        self.add_symbol_reference_to_symbol(
            referencing_symbol.clone(),
            class_member.0.clone(),
            in_signature,
        );

        if in_signature {
            self.symbol_references_to_members_in_signature
                .entry(referencing_symbol)
                .or_insert_with(FxHashSet::default)
                .insert(class_member);
        } else {
            self.symbol_references_to_members
                .entry(referencing_symbol)
                .or_insert_with(FxHashSet::default)
                .insert(class_member);
        }
    }

    pub fn add_symbol_reference_to_symbol(
        &mut self,
        referencing_symbol: StrId,
        symbol: StrId,
        in_signature: bool,
    ) {
        if in_signature {
            self.symbol_references_to_symbols_in_signature
                .entry(referencing_symbol)
                .or_insert_with(FxHashSet::default)
                .insert(symbol);
        } else {
            self.symbol_references_to_symbols
                .entry(referencing_symbol)
                .or_insert_with(FxHashSet::default)
                .insert(symbol);
        }
    }

    pub fn add_class_member_reference_to_class_member(
        &mut self,
        referencing_class_member: (StrId, StrId),
        class_member: (StrId, StrId),
        in_signature: bool,
    ) {
        self.add_symbol_reference_to_symbol(
            referencing_class_member.0.clone(),
            class_member.0.clone(),
            in_signature,
        );

        if in_signature {
            self.classlike_member_references_to_members_in_signature
                .entry(referencing_class_member)
                .or_insert_with(FxHashSet::default)
                .insert(class_member);
        } else {
            self.classlike_member_references_to_members
                .entry(referencing_class_member)
                .or_insert_with(FxHashSet::default)
                .insert(class_member);
        }
    }

    pub fn add_class_member_reference_to_symbol(
        &mut self,
        referencing_class_member: (StrId, StrId),
        symbol: StrId,
        in_signature: bool,
    ) {
        self.add_symbol_reference_to_symbol(
            referencing_class_member.0.clone(),
            symbol.clone(),
            in_signature,
        );

        if in_signature {
            self.classlike_member_references_to_symbols_in_signature
                .entry(referencing_class_member)
                .or_insert_with(FxHashSet::default)
                .insert(symbol);
        } else {
            self.classlike_member_references_to_symbols
                .entry(referencing_class_member)
                .or_insert_with(FxHashSet::default)
                .insert(symbol);
        }
    }

    pub fn add_reference_to_class_member(
        &mut self,
        function_context: &FunctionContext,
        class_member: (StrId, StrId),
        in_signature: bool,
    ) {
        if let Some(referencing_functionlike) = &function_context.calling_functionlike_id {
            match referencing_functionlike {
                FunctionLikeIdentifier::Function(function_name) => self
                    .add_symbol_reference_to_class_member(
                        function_name.clone(),
                        class_member,
                        in_signature,
                    ),
                FunctionLikeIdentifier::Method(class_name, function_name) => self
                    .add_class_member_reference_to_class_member(
                        (class_name.clone(), function_name.clone()),
                        class_member,
                        in_signature,
                    ),
            }
        } else if let Some(calling_class) = &function_context.calling_class {
            self.add_symbol_reference_to_class_member(
                calling_class.clone(),
                class_member,
                in_signature,
            )
        }
    }

    pub fn add_reference_to_overridden_class_member(
        &mut self,
        function_context: &FunctionContext,
        class_member: (StrId, StrId),
    ) {
        if let Some(referencing_functionlike) = &function_context.calling_functionlike_id {
            match referencing_functionlike {
                FunctionLikeIdentifier::Function(function_name) => {
                    self.symbol_references_to_overridden_members
                        .entry(function_name.clone())
                        .or_insert_with(FxHashSet::default)
                        .insert(class_member);
                }
                FunctionLikeIdentifier::Method(class_name, function_name) => {
                    self.classlike_member_references_to_overridden_members
                        .entry((class_name.clone(), function_name.clone()))
                        .or_insert_with(FxHashSet::default)
                        .insert(class_member);
                }
            }
        } else if let Some(calling_class) = &function_context.calling_class {
            self.symbol_references_to_overridden_members
                .entry(calling_class.clone())
                .or_insert_with(FxHashSet::default)
                .insert(class_member);
        }
    }

    pub fn add_reference_to_symbol(
        &mut self,
        function_context: &FunctionContext,
        symbol: StrId,
        in_signature: bool,
    ) {
        if let Some(referencing_functionlike) = &function_context.calling_functionlike_id {
            match referencing_functionlike {
                FunctionLikeIdentifier::Function(function_name) => {
                    self.add_symbol_reference_to_symbol(function_name.clone(), symbol, in_signature)
                }
                FunctionLikeIdentifier::Method(class_name, function_name) => self
                    .add_class_member_reference_to_symbol(
                        (class_name.clone(), function_name.clone()),
                        symbol,
                        in_signature,
                    ),
            }
        } else if let Some(calling_class) = &function_context.calling_class {
            self.add_symbol_reference_to_symbol(calling_class.clone(), symbol, in_signature)
        }
    }

    pub fn add_reference_to_functionlike_return(
        &mut self,
        referencing_functionlike: FunctionLikeIdentifier,
        functionlike: FunctionLikeIdentifier,
    ) {
        self.functionlike_references_to_functionlike_returns
            .entry(referencing_functionlike)
            .or_insert_with(FxHashSet::default)
            .insert(functionlike);
    }

    pub fn extend(&mut self, other: Self) {
        for (k, v) in other.symbol_references_to_members {
            self.symbol_references_to_members
                .entry(k)
                .or_insert_with(FxHashSet::default)
                .extend(v);
        }

        for (k, v) in other.symbol_references_to_symbols {
            self.symbol_references_to_symbols
                .entry(k)
                .or_insert_with(FxHashSet::default)
                .extend(v);
        }

        for (k, v) in other.classlike_member_references_to_symbols {
            self.classlike_member_references_to_symbols
                .entry(k)
                .or_insert_with(FxHashSet::default)
                .extend(v);
        }

        for (k, v) in other.classlike_member_references_to_members {
            self.classlike_member_references_to_members
                .entry(k)
                .or_insert_with(FxHashSet::default)
                .extend(v);
        }

        for (k, v) in other.symbol_references_to_members_in_signature {
            self.symbol_references_to_members_in_signature
                .entry(k)
                .or_insert_with(FxHashSet::default)
                .extend(v);
        }

        for (k, v) in other.symbol_references_to_symbols_in_signature {
            self.symbol_references_to_symbols_in_signature
                .entry(k)
                .or_insert_with(FxHashSet::default)
                .extend(v);
        }

        for (k, v) in other.classlike_member_references_to_symbols_in_signature {
            self.classlike_member_references_to_symbols_in_signature
                .entry(k)
                .or_insert_with(FxHashSet::default)
                .extend(v);
        }

        for (k, v) in other.classlike_member_references_to_members_in_signature {
            self.classlike_member_references_to_members_in_signature
                .entry(k)
                .or_insert_with(FxHashSet::default)
                .extend(v);
        }

        for (k, v) in other.symbol_references_to_overridden_members {
            self.symbol_references_to_overridden_members
                .entry(k)
                .or_insert_with(FxHashSet::default)
                .extend(v);
        }

        for (k, v) in other.classlike_member_references_to_overridden_members {
            self.classlike_member_references_to_overridden_members
                .entry(k)
                .or_insert_with(FxHashSet::default)
                .extend(v);
        }
    }

    pub fn get_referenced_symbols(&self) -> FxHashSet<&StrId> {
        let mut referenced_symbols = FxHashSet::default();

        for (_, symbol_references_to_symbols) in &self.symbol_references_to_symbols {
            referenced_symbols.extend(symbol_references_to_symbols);
        }

        for (_, symbol_references_to_symbols) in &self.symbol_references_to_symbols_in_signature {
            referenced_symbols.extend(symbol_references_to_symbols);
        }

        referenced_symbols
    }

    pub fn get_referenced_class_members(&self) -> FxHashSet<&(StrId, StrId)> {
        let mut referenced_class_members = FxHashSet::default();

        for (_, symbol_references_to_class_members) in &self.symbol_references_to_members {
            referenced_class_members.extend(symbol_references_to_class_members);
        }

        for (_, class_member_references_to_class_members) in
            &self.classlike_member_references_to_members
        {
            referenced_class_members.extend(class_member_references_to_class_members);
        }

        for (_, symbol_references_to_class_members) in
            &self.symbol_references_to_members_in_signature
        {
            referenced_class_members.extend(symbol_references_to_class_members);
        }

        for (_, class_member_references_to_class_members) in
            &self.classlike_member_references_to_members_in_signature
        {
            referenced_class_members.extend(class_member_references_to_class_members);
        }

        referenced_class_members
    }

    pub fn get_referenced_overridden_class_members(&self) -> FxHashSet<&(StrId, StrId)> {
        let mut referenced_class_members = FxHashSet::default();

        for (_, symbol_references_to_class_members) in &self.symbol_references_to_overridden_members
        {
            referenced_class_members.extend(symbol_references_to_class_members);
        }

        for (_, class_member_references_to_class_members) in
            &self.classlike_member_references_to_overridden_members
        {
            referenced_class_members.extend(class_member_references_to_class_members);
        }

        referenced_class_members
    }

    pub fn get_invalid_symbols(
        &self,
        codebase_diff: &CodebaseDiff,
    ) -> (
        FxHashSet<StrId>,
        FxHashSet<(StrId, StrId)>,
        FxHashSet<StrId>,
    ) {
        let mut invalid_symbols = FxHashSet::default();
        let mut invalid_symbol_members = FxHashSet::default();

        let mut new_invalid_symbols = codebase_diff.add_or_delete.clone();

        let mut seen_symbols = FxHashSet::default();

        while !new_invalid_symbols.is_empty() {
            let new_invalid_symbol = new_invalid_symbols.pop().unwrap();

            if seen_symbols.contains(&new_invalid_symbol) {
                continue;
            }

            seen_symbols.insert(new_invalid_symbol);

            let (changed_symbol, changed_symbol_member) = new_invalid_symbol;

            if let Some(changed_symbol_member) = changed_symbol_member {
                for (referencing_member, referenced_members) in
                    &self.classlike_member_references_to_members_in_signature
                {
                    if referenced_members.contains(&(changed_symbol, changed_symbol_member)) {
                        new_invalid_symbols
                            .push((referencing_member.0, Some(referencing_member.1)));
                        invalid_symbol_members.insert(*referencing_member);
                    }
                }

                for (referencing_member, referenced_members) in
                    &self.symbol_references_to_members_in_signature
                {
                    if referenced_members.contains(&(changed_symbol, changed_symbol_member)) {
                        new_invalid_symbols.push((*referencing_member, None));
                        invalid_symbols.insert(*referencing_member);
                    }
                }

                invalid_symbol_members.insert((changed_symbol, changed_symbol_member));
            } else {
                for (referencing_member, referenced_members) in
                    &self.classlike_member_references_to_symbols_in_signature
                {
                    if referenced_members.contains(&changed_symbol) {
                        new_invalid_symbols
                            .push((referencing_member.0, Some(referencing_member.1)));
                        invalid_symbol_members.insert(*referencing_member);
                    }
                }

                for (referencing_member, referenced_members) in
                    &self.symbol_references_to_symbols_in_signature
                {
                    if referenced_members.contains(&changed_symbol) {
                        new_invalid_symbols.push((*referencing_member, None));
                        invalid_symbols.insert(*referencing_member);
                    }
                }

                invalid_symbols.insert(changed_symbol);
            }
        }

        let mut invalid_symbol_bodies = FxHashSet::default();
        let mut invalid_symbol_member_bodies = FxHashSet::default();

        for invalid_symbol in &invalid_symbols {
            for (referencing_member, referenced_members) in
                &self.classlike_member_references_to_symbols
            {
                if referenced_members.contains(invalid_symbol) {
                    invalid_symbol_member_bodies.insert(*referencing_member);
                }
            }

            for (referencing_member, referenced_members) in &self.symbol_references_to_symbols {
                if referenced_members.contains(invalid_symbol) {
                    invalid_symbol_bodies.insert(*referencing_member);
                }
            }
        }

        for invalid_symbol_member in &invalid_symbol_members {
            for (referencing_member, referenced_members) in
                &self.classlike_member_references_to_members
            {
                if referenced_members.contains(&(invalid_symbol_member.0, invalid_symbol_member.1))
                {
                    invalid_symbol_member_bodies.insert(*referencing_member);
                }
            }

            for (referencing_member, referenced_members) in &self.symbol_references_to_members {
                if referenced_members.contains(&(invalid_symbol_member.0, invalid_symbol_member.1))
                {
                    invalid_symbol_bodies.insert(*referencing_member);
                }
            }
        }

        invalid_symbols.extend(invalid_symbol_bodies);
        invalid_symbol_members.extend(invalid_symbol_member_bodies);

        let partially_invalid_symbols = invalid_symbol_members
            .iter()
            .map(|(a, _)| *a)
            .collect::<FxHashSet<_>>();

        for keep_signature in &codebase_diff.keep_signature {
            if let Some(member_id) = keep_signature.1 {
                invalid_symbol_members.insert((keep_signature.0, member_id));
            } else {
                invalid_symbols.insert(keep_signature.0);
            }
        }

        (
            invalid_symbols,
            invalid_symbol_members,
            partially_invalid_symbols,
        )
    }

    pub fn get_ranked_functions(&self, codebase: &CodebaseInfo) -> Vec<FunctionLikeIdentifier> {
        let mut functionlike_ids = codebase
            .functionlike_infos
            .iter()
            .filter(|(_, v)| v.is_production_code)
            .map(|(k, _)| (k, None))
            .collect::<Vec<_>>();

        functionlike_ids.extend(
            codebase
                .classlike_infos
                .iter()
                .filter(|(_, v)| v.is_production_code)
                .map(|c| {
                    c.1.methods
                        .keys()
                        .into_iter()
                        .map(|m| (c.0, Some(m)))
                        .collect::<Vec<_>>()
                })
                .flatten(),
        );

        println!("building matrix of size {}", functionlike_ids.len());

        let matrix = self.build_matrix(&functionlike_ids);

        println!("calculating rank");

        let mut ranked_ids = calculate_rank(matrix, 100)
            .into_iter()
            .enumerate()
            .map(|e| (functionlike_ids[e.0], e.1))
            .filter(|e| e.1 > 0.0)
            .collect::<Vec<_>>();

        ranked_ids.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        ranked_ids
            .into_iter()
            .map(|(id, _)| {
                if let Some(member_id) = id.1 {
                    FunctionLikeIdentifier::Method(*id.0, *member_id)
                } else {
                    FunctionLikeIdentifier::Function(*id.0)
                }
            })
            .collect()
    }

    fn build_matrix(&self, functionlike_ids: &Vec<(&StrId, Option<&StrId>)>) -> Array2<f32> {
        let len = functionlike_ids.len();
        let mut matrix = Array2::<f32>::zeros((len, len));

        for j in 0..len {
            let source = functionlike_ids[j];

            let refs = match source.1 {
                Some(source_member_id) => (
                    self.classlike_member_references_to_symbols
                        .get(&(*source.0, *source_member_id)),
                    self.classlike_member_references_to_members
                        .get(&(*source.0, *source_member_id)),
                ),
                _ => (
                    self.symbol_references_to_symbols.get(&source.0),
                    self.symbol_references_to_members.get(&source.0),
                ),
            };

            if j % 10000 == 0 {
                println!("{}", j);
            }

            if let (None, None) = refs {
                continue;
            }

            for i in 0..len {
                if i == j {
                    continue;
                }

                let target = functionlike_ids[i];

                if match target.1 {
                    Some(target_member_id) => {
                        if let Some(refs) = refs.1 {
                            refs.contains(&(*target.0, *target_member_id))
                        } else {
                            false
                        }
                    }
                    _ => {
                        if let Some(refs) = refs.0 {
                            refs.contains(&target.0)
                        } else {
                            false
                        }
                    }
                } {
                    matrix[[i, j]] = 1.0;
                };
            }
        }

        println!("Summing columns");

        let sum_column = matrix.sum_axis(ndarray::Axis(0)).to_vec();

        println!("Normalizing matrix");

        for i in 0..len {
            if i % 10000 == 0 {
                println!("{}", i);
            }

            for j in 0..len {
                if i == j {
                    continue;
                }
                if sum_column[j] != 0.0 {
                    matrix[[i, j]] = matrix[[i, j]] / sum_column[j];
                }
            }
        }

        matrix
    }

    pub fn remove_references_from_invalid_symbols(
        &mut self,
        invalid_symbols: &FxHashSet<StrId>,
        invalid_symbol_members: &FxHashSet<(StrId, StrId)>,
    ) {
        self.symbol_references_to_members
            .retain(|symbol, _| !invalid_symbols.contains(symbol));
        self.symbol_references_to_members_in_signature
            .retain(|symbol, _| !invalid_symbols.contains(symbol));
        self.symbol_references_to_symbols
            .retain(|symbol, _| !invalid_symbols.contains(symbol));
        self.symbol_references_to_symbols_in_signature
            .retain(|symbol, _| !invalid_symbols.contains(symbol));

        self.classlike_member_references_to_members
            .retain(|symbol, _| !invalid_symbol_members.contains(symbol));
        self.classlike_member_references_to_members_in_signature
            .retain(|symbol, _| !invalid_symbol_members.contains(symbol));
        self.classlike_member_references_to_symbols
            .retain(|symbol, _| !invalid_symbol_members.contains(symbol));
        self.classlike_member_references_to_symbols_in_signature
            .retain(|symbol, _| !invalid_symbol_members.contains(symbol));
    }
}

fn calculate_rank(similarity_matrix: Array2<f32>, limit: usize) -> Vec<f32> {
    let edges_count = similarity_matrix.shape()[1];
    let threshold = 0.001;
    // Initialize a vector with the same value 1/number of sentences. Uniformly distributed across
    // all sentences. NOTE: perhaps we can make some sentences more important than the rest?
    let initial_vector: Vec<f32> = vec![1.0 / edges_count as f32; edges_count];
    let mut result = Array1::from(initial_vector);
    let mut prev_result = result.clone();
    let damping_factor = 0.85;
    let initial_m =
        damping_factor * similarity_matrix + (1.0 - damping_factor) / edges_count as f32;
    for _ in 0..limit {
        println!("looping");
        result = initial_m.dot(&result);
        let delta = &result - &prev_result;
        let mut converged = true;
        for i in 0..delta.len() {
            if delta[i] > threshold {
                converged = false;
                break;
            }
        }
        if converged {
            break;
        }
        prev_result = result.clone();
    }
    result.into_raw_vec()
}
