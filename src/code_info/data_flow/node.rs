use std::hash::{Hash, Hasher};

use crate::function_context::FunctionLikeIdentifier;
use crate::method_identifier::MethodIdentifier;
use crate::{
    code_location::HPos,
    taint::{SinkType, SourceType},
};
use hakana_str::{Interner, StrId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableSourceKind {
    Default,
    PrivateParam,
    NonPrivateParam,
    InoutParam,
    ClosureParam,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataFlowNode {
    pub id: String,
    pub kind: DataFlowNodeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataFlowNodeKind {
    Vertex {
        pos: Option<HPos>,
        unspecialized_id: Option<String>,
        label: String,
        specialization_key: Option<(StrId, u32)>,
    },
    VariableUseSource {
        pos: HPos,
        kind: VariableSourceKind,
        label: String,
        pure: bool,
        has_parent_nodes: bool,
        has_awaitable: bool,
    },
    VariableUseSink {
        pos: HPos,
    },
    ForLoopInit {
        var_name: String,
        start_offset: u32,
        end_offset: u32,
    },
    DataSource {
        pos: HPos,
        label: String,
        target_id: String,
    },
    TaintSource {
        pos: Option<HPos>,
        label: String,
        types: Vec<SourceType>,
    },
    TaintSink {
        pos: Option<HPos>,
        label: String,
        types: Vec<SinkType>,
    },
}

impl Hash for DataFlowNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl DataFlowNode {
    pub fn new(
        id: String,
        label: String,
        pos: Option<HPos>,
        specialization_key: Option<(StrId, u32)>,
    ) -> Self {
        let mut id = id;
        let mut unspecialized_id = None;

        if let Some(specialization_key) = &specialization_key {
            unspecialized_id = Some(id.clone());
            id += "-";
            id += &specialization_key.0 .0.to_string();
            id += ":";
            id += &specialization_key.1.to_string();
        }

        DataFlowNode {
            id,
            kind: DataFlowNodeKind::Vertex {
                pos,
                unspecialized_id,
                label,
                specialization_key,
            },
        }
    }

    pub fn get_for_method_argument(
        method_id: String,
        argument_offset: usize,
        arg_location: Option<HPos>,
        pos: Option<HPos>,
    ) -> Self {
        let arg_id = method_id.clone() + "#" + (argument_offset + 1).to_string().as_str();

        let mut specialization_key = None;

        if let Some(pos) = pos {
            specialization_key = Some((pos.file_path.0, pos.start_offset));
        }

        Self::new(arg_id.clone(), arg_id, arg_location, specialization_key)
    }

    pub fn get_for_method_argument_out(
        method_id: String,
        argument_offset: usize,
        arg_location: Option<HPos>,
        pos: Option<HPos>,
    ) -> Self {
        let arg_id = "out ".to_string()
            + method_id.as_str()
            + "#"
            + (argument_offset + 1).to_string().as_str();

        let mut specialization_key = None;

        if let Some(pos) = pos {
            specialization_key = Some((pos.file_path.0, pos.start_offset));
        }

        Self::new(arg_id.clone(), arg_id, arg_location, specialization_key)
    }

    pub fn get_for_this_before_method(
        method_id: &MethodIdentifier,
        method_location: Option<HPos>,
        pos: Option<HPos>,
        interner: &Interner,
    ) -> Self {
        let label = format!(
            "$this in {} before {}",
            interner.lookup(&method_id.0),
            interner.lookup(&method_id.1)
        );

        let mut specialization_key = None;

        if let Some(pos) = pos {
            specialization_key = Some((pos.file_path.0, pos.start_offset));
        }

        DataFlowNode::new(label.clone(), label, method_location, specialization_key)
    }

    pub fn get_for_this_after_method(
        method_id: &MethodIdentifier,
        method_location: Option<HPos>,
        pos: Option<HPos>,
        interner: &Interner,
    ) -> Self {
        let label = format!(
            "$this in {} after {}",
            interner.lookup(&method_id.0),
            interner.lookup(&method_id.1)
        );

        let mut specialization_key = None;

        if let Some(pos) = pos {
            specialization_key = Some((pos.file_path.0, pos.start_offset));
        }

        DataFlowNode::new(label.clone(), label, method_location, specialization_key)
    }

    pub fn get_for_lvar(var_id: String, assignment_location: HPos, has_parent_nodes: bool) -> Self {
        let id = format!(
            "{}-{}:{}-{}",
            var_id,
            assignment_location.file_path.0 .0,
            assignment_location.start_offset,
            assignment_location.end_offset
        );

        DataFlowNode {
            id,
            kind: DataFlowNodeKind::Vertex {
                pos: Some(assignment_location),
                unspecialized_id: None,
                label: var_id,
                specialization_key: None,
            },
        }
    }

    pub fn get_for_return_expr(
        var_id: String,
        assignment_location: HPos,
        has_parent_nodes: bool,
    ) -> Self {
        let id = format!(
            "{}-{}:{}-{}",
            var_id,
            assignment_location.file_path.0 .0,
            assignment_location.start_offset,
            assignment_location.end_offset
        );

        Self::new(id, var_id, Some(assignment_location), None)
    }

    pub fn get_for_array_item(
        var_id: String,
        assignment_location: HPos,
        has_parent_nodes: bool,
    ) -> Self {
        let id = format!(
            "{}-{}:{}-{}",
            var_id,
            assignment_location.file_path.0 .0,
            assignment_location.start_offset,
            assignment_location.end_offset
        );

        Self::new(id, var_id, Some(assignment_location), None)
    }

    pub fn get_for_array_fetch(var_id: String, assignment_location: HPos) -> Self {
        let id = format!(
            "{}-{}:{}-{}",
            var_id,
            assignment_location.file_path.0 .0,
            assignment_location.start_offset,
            assignment_location.end_offset
        );

        Self::new(id, var_id, Some(assignment_location), None)
    }

    pub fn get_for_property_fetch(var_id: String, assignment_location: HPos) -> Self {
        let id = format!(
            "{}-{}:{}-{}",
            var_id,
            assignment_location.file_path.0 .0,
            assignment_location.start_offset,
            assignment_location.end_offset
        );

        Self::new(id, var_id, Some(assignment_location), None)
    }

    pub fn get_for_narrowing(var_id: String, assignment_location: HPos) -> Self {
        let id = format!(
            "{}-{}:{}-{}",
            var_id,
            assignment_location.file_path.0 .0,
            assignment_location.start_offset,
            assignment_location.end_offset
        );

        Self::new(id, var_id, Some(assignment_location), None)
    }

    pub fn get_for_instance_property_assignment(
        lhs_var_id: &String,
        property_name: StrId,
        interner: &Interner,
        assignment_location: HPos,
        has_parent_nodes: bool,
    ) -> Self {
        let var_id = format!("{}->{}", lhs_var_id, interner.lookup(&property_name));
        let id = format!(
            "{}-{}:{}-{}",
            var_id,
            assignment_location.file_path.0 .0,
            assignment_location.start_offset,
            assignment_location.end_offset
        );

        Self::new(id, var_id, Some(assignment_location), None)
    }

    pub fn get_for_unspecialized_property(
        property_id: (StrId, StrId),
        interner: &Interner,
        assignment_location: HPos,
    ) -> Self {
        let property_id = format!(
            "{}::${}",
            interner.lookup(&property_id.0),
            interner.lookup(&property_id.1)
        );

        let id = format!(
            "{}-{}:{}-{}",
            property_id,
            assignment_location.file_path.0 .0,
            assignment_location.start_offset,
            assignment_location.end_offset
        );

        Self::new(id, property_id, Some(assignment_location), None)
    }

    pub fn get_for_call(
        functionlike_id: FunctionLikeIdentifier,
        interner: &Interner,
        assignment_location: HPos,
    ) -> Self {
        let label = format!("call to {}", functionlike_id.to_string(interner));
        let id = format!(
            "{}-{}:{}-{}",
            label,
            assignment_location.file_path.0 .0,
            assignment_location.start_offset,
            assignment_location.end_offset
        );

        Self::new(id, label, Some(assignment_location), None)
    }

    pub fn get_for_composition(assignment_location: HPos) -> Self {
        let id = format!(
            "composition-{}:{}-{}",
            assignment_location.file_path.0 .0,
            assignment_location.start_offset,
            assignment_location.end_offset
        );

        Self::new(
            id.clone(),
            "composition".to_string(),
            Some(assignment_location),
            None,
        )
    }

    pub fn get_for_variable_sink(label: String, assignment_location: HPos) -> Self {
        let id = format!(
            "{}-{}:{}-{}",
            label,
            assignment_location.file_path.0 .0,
            assignment_location.start_offset,
            assignment_location.end_offset
        );

        Self {
            id,
            kind: DataFlowNodeKind::VariableUseSink {
                pos: assignment_location,
            },
        }
    }

    pub fn get_for_variable_source(
        label: String,
        assignment_location: HPos,
        pure: bool,
        has_parent_nodes: bool,
        has_awaitable: bool,
    ) -> Self {
        let id = format!(
            "{}-{}:{}-{}",
            label,
            assignment_location.file_path.0 .0,
            assignment_location.start_offset,
            assignment_location.end_offset
        );

        Self {
            id,
            kind: DataFlowNodeKind::VariableUseSource {
                pos: assignment_location,
                kind: VariableSourceKind::Default,
                label,
                pure,
                has_awaitable,
                has_parent_nodes,
            },
        }
    }

    pub fn get_for_method_return(
        method_id: String,
        pos: Option<HPos>,
        specialization_location: Option<HPos>,
    ) -> Self {
        let mut specialization_key = None;

        if let Some(specialization_location) = specialization_location {
            specialization_key = Some((
                (specialization_location.file_path).0,
                specialization_location.start_offset,
            ));
        }

        Self::new(
            method_id.clone(),
            format!("{}()", method_id),
            pos,
            specialization_key,
        )
    }

    pub fn get_for_method_reference(method_id: String, pos: Option<HPos>) -> Self {
        Self::new(
            format!("fnref-{}", method_id),
            format!("{}()", method_id),
            pos,
            None,
        )
    }

    #[inline]
    pub fn get_id(&self) -> &String {
        &self.id
    }

    #[inline]
    pub fn get_label(&self) -> &String {
        match &self.kind {
            DataFlowNodeKind::Vertex { label, .. }
            | DataFlowNodeKind::TaintSource { label, .. }
            | DataFlowNodeKind::TaintSink { label, .. }
            | DataFlowNodeKind::VariableUseSource { label, .. }
            | DataFlowNodeKind::DataSource { label, .. } => label,
            DataFlowNodeKind::ForLoopInit { .. } | DataFlowNodeKind::VariableUseSink { .. } => {
                &self.id
            }
        }
    }

    #[inline]
    pub fn get_pos(&self) -> &Option<HPos> {
        match &self.kind {
            DataFlowNodeKind::Vertex { pos, .. }
            | DataFlowNodeKind::TaintSource { pos, .. }
            | DataFlowNodeKind::TaintSink { pos, .. } => pos,
            DataFlowNodeKind::VariableUseSource { .. }
            | DataFlowNodeKind::ForLoopInit { .. }
            | DataFlowNodeKind::VariableUseSink { .. }
            | DataFlowNodeKind::DataSource { .. } => {
                panic!()
            }
        }
    }
}
