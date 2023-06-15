use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{ast_signature::DefSignatureNode, functionlike_info::FunctionLikeInfo, StrId};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UsesFlippedMap {
    pub type_aliases_flipped: FxHashMap<StrId, StrId>,
    pub namespace_aliases_flipped: FxHashMap<StrId, StrId>,
    pub const_aliases_flipped: FxHashMap<StrId, StrId>,
    pub fun_aliases_flipped: FxHashMap<StrId, StrId>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FileInfo {
    pub ast_nodes: Vec<DefSignatureNode>,
    pub closure_infos: FxHashMap<usize, FunctionLikeInfo>,
    pub uses_flipped_map: UsesFlippedMap,
}
