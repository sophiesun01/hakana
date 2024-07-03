extern crate json_value_merge;

use std::collections::VecDeque;
use oxidized::{
    aast,
    aast_visitor::{AstParams, Node, Visitor},
    ast_defs,
    aast_defs
};
use serde_json::{self, json};
use serde_json::Value;

pub(crate) struct Scanner {
    pub tree_stack: Vec<Value>,
    pub tree: String,
    pub string_stack: Vec<String>,
    pub show_pos: bool
}
pub(crate) struct Context {

}

fn visit_pos(
    pos: ast_defs::Pos,
    show_pos: bool,
    ) -> Value {
        if show_pos{
            let (start, end) = pos.to_start_and_end_lnum_bol_offset();
            let pos = json!({
                "kind": "Pos",
                "file": pos.filename(),
                "startLine": start.0,
                "startBol": start.1,
                "startOffset": start.2,
                "endLine": end.0,
                "endBol": end.1,
                "endOffset": end.2,
            });
            pos
        }
        else{
            Value::Null
        }

}

fn get_vec_len(tree_stack: &mut Vec<Value>, kind: &str, mut n: usize
    )-> VecDeque<Value> {

let mut arr = VecDeque::new();
while let Some(last) = tree_stack.last() {
    if last["kind"] != kind || n == 0 {
        if n > 0{
            // println!("Something ain't right {}", kind);
        }
        break;
    }

    n -= 1;
    arr.push_front(tree_stack.pop().unwrap());
}
arr
}


impl <'ast>Visitor<'ast> for Scanner {
	type Params = AstParams<Context, ()>;
	fn object(&mut self) -> &mut dyn Visitor<'ast, Params = Self::Params> {
        self
	}
    //Skip
    fn visit_ex(
        &mut self,
        _c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        _p: &'ast <Self::Params as oxidized::aast_visitor::Params>::Ex,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        Ok(())
    }
    //Skip
    fn visit_en(
        &mut self,
        _c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        _p: &'ast <Self::Params as oxidized::aast_visitor::Params>::En,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        Ok(())
    }
    // Used
    fn visit_abstraction(
        &mut self,
        _c: &mut Context,
        p: &'ast ast_defs::Abstraction,
    ) -> Result<(), ()> {
        p.recurse(_c, self.object())
    }
    //PLEASE IMPLEMENT EVENTUALLY
    fn visit_afield(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::Afield<(), ()>,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    fn visit_as_expr(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::AsExpr<(), ()>,
    ) -> Result<(), ()> {
        // println!("As Expr");
        let _ = p.recurse(_c, self.object());
        let ae = match p{
            aast::AsExpr::AsV(_) =>{
                json!({
                    "kind": "AsExpr",
                    "type": "AsV",
                    "asExpr": self.tree_stack.pop(),
                })
            }
            aast::AsExpr::AsKv(_, _) =>{
                let e2 = self.tree_stack.pop();
                let e1 = self.tree_stack.pop();
                json!({
                    "kind": "AsExpr",
                    "type": "AsKv",
                    "asExpr": [e1, e2],
                })
            }
            aast::AsExpr::AwaitAsV(_, _) =>{
                json!({
                    "kind": "AsExpr",
                    "type": "AwaitAsV",
                    "asExpr": self.tree_stack.pop(),
                })
            }
            aast::AsExpr::AwaitAsKv(_, _, _) =>{
                let e2 = self.tree_stack.pop();
                let e1 = self.tree_stack.pop();

                json!({
                    "kind": "AsExpr",
                    "type": "AwaitAsKv",
                    "asExpr": [e1, e2],
                })
            }
        };
        self.tree_stack.push(ae);
        Ok(())
    }
    fn visit_as_(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::As_<(), ()>,
    ) -> Result<(), ()> {
        let hint = self.tree_stack.pop();
        let expr = self.tree_stack.pop();
        let _ = p.recurse(_c, self.object());
        let as_ = json!({
            "kind": "As",
            "typeHint": hint,
            "expr": expr,
        });
        self.tree_stack.push(as_);
        Ok(())
    }

    fn visit_binop(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Binop<(), ()>,
    ) -> Result<(), ()> {
        // println!{"Binop"};
        let _ = p.recurse(_c, self.object());
        let rhs = self.tree_stack.pop();
        let lhs = self.tree_stack.pop();
        let bi = json!({
			"kind": "Binop",
            "bop": self.string_stack.pop(),
			"lhs": lhs,
            "rhs": rhs,
		});
        self.tree_stack.push(bi);
        Ok(())
    }

    fn visit_block(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Block<(), ()>,
    ) -> Result<(), ()> {
        // println!("Block");
        let _  = p.recurse(_c, self.object());
        let stmts = get_vec_len(&mut self.tree_stack, "Stmt", p.0.len());
        let b = json!({
            "kind": "Block",
            "stmts": stmts,
        });
        self.tree_stack.push(b);
        Ok(())
    }

    fn visit_bop(
        &mut self,
        _c: &mut Context,
        p: &'ast ast_defs::Bop,
    ) -> Result<(), ()> {
        // println!("Bop");
        let _ = p.recurse(_c, self.object());
        let bop = match p{
            ast_defs::Bop::Plus => "Plus".to_string(),
            ast_defs::Bop::Minus => "Minus".to_string(),
            ast_defs::Bop::Star => "Star".to_string(),
            ast_defs::Bop::Slash => "Slash".to_string(),
            ast_defs::Bop::Eqeq => "Eqeq".to_string(),
            ast_defs::Bop::Eqeqeq => "Eqeqeq".to_string(),
            ast_defs::Bop::Starstar => "Starstar".to_string(),
            ast_defs::Bop::Diff => "Diff".to_string(),
            ast_defs::Bop::Diff2 => "Diff2".to_string(),
            ast_defs::Bop::Ampamp => "Ampamp".to_string(),
            ast_defs::Bop::Barbar => "Barbar".to_string(),
            ast_defs::Bop::Lt => "Lt".to_string(),
            ast_defs::Bop::Lte => "Lte".to_string(),
            ast_defs::Bop::Gt => "Gt".to_string(),
            ast_defs::Bop::Gte => "Gte".to_string(),
            ast_defs::Bop::Dot => "Dot".to_string(),
            ast_defs::Bop::Amp => "Amp".to_string(),
            ast_defs::Bop::Bar => "Bar".to_string(),
            ast_defs::Bop::Ltlt => "Ltlt".to_string(),
            ast_defs::Bop::Gtgt => "Gtgt".to_string(),
            ast_defs::Bop::Percent => "Percent".to_string(),
            ast_defs::Bop::Xor => "Xor".to_string(),
            ast_defs::Bop::Cmp => "Cmp".to_string(),
            ast_defs::Bop::QuestionQuestion => "QuestionQuestion".to_string(),
            //Test this case
            ast_defs::Bop::Eq(a) => {
                if a.is_some() {
                    let val = self.string_stack.pop().unwrap_or_default() + "Eq";
                    val
                } else {
                    "Eq".to_string()
                }
            },
        };
        self.string_stack.push(bop);
        Ok(())
    }
    fn visit_call_expr(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::CallExpr<(), ()>,
    ) -> Result<(), ()> {
        // println!("Call Expr");
        let _ = p.recurse(_c, self.object());

        let mut unpacked_arg = Value::Null;
        if p.unpacked_arg.is_some(){
            unpacked_arg = self.tree_stack.pop().unwrap();
        }
        let args = get_vec_len(&mut self.tree_stack,"Expr", p.args.len());
        let func = self.tree_stack.pop();
        if p.unpacked_arg.is_none(){
            let ce = json!({
                "kind": "CallExpr",
                "func": func,
                "args": args,
            });
            self.tree_stack.push(ce);
        }
        else{
            let ce = json!({
                "kind": "CallExpr",
                "func": self.tree_stack.pop(),
                "args": args,
                "unpacked_args": unpacked_arg,
            });
            self.tree_stack.push(ce);
        }
        Ok(())
    }
    // PLEASE IMPLEMENT EVENTUALLY
    fn visit_capture_lid(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::CaptureLid<()>,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    fn visit_case(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Case<(), ()>,
    ) -> Result<(), ()> {
        // println!("Case");
        let _ = p.recurse(_c, self.object());
        let block = self.tree_stack.pop();
        let expr = self.tree_stack.pop();
        let c = json!({
            "kind": "Case",
            "expr": expr,
            "block": block, 
        });
        self.tree_stack.push(c);
        Ok(())
    }
    fn visit_catch(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Catch<(), ()>,
    ) -> Result<(), ()> {
        // println!("Catch");
        let _ = p.recurse(_c, self.object());
        let c = json!({
            "kind": "Catch",
            "class": p.0.1.clone(),
            "id": p.1.1.clone(),
            "block": self.tree_stack.pop(), 
        });
        self.tree_stack.push(c);
        Ok(())
    }
    // Skip
    fn visit_class_abstract_typeconst(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::ClassAbstractTypeconst,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    // Skip
    fn visit_class_concrete_typeconst(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::ClassConcreteTypeconst,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    // PLEASE IMPLEMENT EVENTUALLY
    fn visit_class_const(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::ClassConst<(), ()>,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    // PLEASE IMPLEMENT EVENTUALLY
    fn visit_class_const_kind(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::ClassConstKind<(), ()>,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    // PLEASE IMPLEMENT EVENTUALLY
    fn visit_class_get_expr(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::ClassGetExpr<(), ()>,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    // USED
    fn visit_class_id(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::ClassId<(), ()>,
    ) -> Result<(), ()> {
        let _ = p.recurse(_c, self.object());
        Ok(())
    }
    fn visit_class_id_(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::ClassId_<(), ()>,
    ) -> Result<(), ()> {
        let _ = p.recurse(_c, self.object());
        match p{
            aast::ClassId_::CIparent =>{
                let cid = json!({
                    "kind": "ClassId",
                    "type": "Parent",
                });
                self.tree_stack.push(cid); 
            }
            aast::ClassId_::CIself =>{
                let cid = json!({
                    "kind": "ClassId",
                    "type": "Self",
                });
                self.tree_stack.push(cid); 
            }
            aast::ClassId_::CIstatic =>{
                let cid = json!({
                    "kind": "ClassId",
                    "type": "Static",
                });
                self.tree_stack.push(cid); 
            }
            aast::ClassId_::CIexpr(_) =>{
                let cid = json!({
                    "kind": "ClassId",
                    "type": "Dynamic",
                    "expr": self.tree_stack.pop(),
                });
                self.tree_stack.push(cid); 
            }
            aast::ClassId_::CI(a) =>{
                let cid = json!({
                    "kind": "ClassId",
                    "type": "Explicit",
                    "expr": a.1.clone(),
                });
                self.tree_stack.push(cid); 
            }
        }
        Ok(())
    }
    // Skip
    fn visit_class_req(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::ClassReq,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    // Skip
    fn visit_class_typeconst(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::ClassTypeconst,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    // Skip
    fn visit_class_typeconst_def(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::ClassTypeconstDef<(), ()>,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }

    fn visit_class_var(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::ClassVar<(), ()>,
    ) -> Result<(), ()> {
        // println!("Class Var");
        let _ = p.recurse(_c, self.object());
        let mut expr = Value::Null;
        if p.expr.is_some(){
            expr = self.tree_stack.pop().unwrap();
        }
        let type_hint = self.tree_stack.pop();
        let cv = json!({
            "kind": "ClassVar",
            "abstract": p.abstract_.clone(),
            "readonly": p.readonly.clone(),
            "visibility": self.string_stack.pop(),
            "type_hint": type_hint,
            "name": p.id.1.clone(),
            "span": visit_pos(p.id.0.clone(), self.show_pos),
            "expr": expr,
            "doc_comment": p.doc_comment.clone(),
            "is_static": p.is_static.clone()
        });
        self.tree_stack.push(cv);
        Ok(())
    }

    fn visit_class_(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Class_<(), ()>,
    ) -> Result<(), ()> {
        // println!("Class");
        let _ = p.recurse(_c, self.object());
  
        let methods = get_vec_len(&mut self.tree_stack, "Method", p.methods.len());
        let vars = get_vec_len(&mut self.tree_stack, "ClassVar", p.vars.len());
        let where_constraints = get_vec_len(&mut self.tree_stack, "WhereConstraintHint", p.where_constraints.len());
        let implements = get_vec_len(&mut self.tree_stack, "Hint", p.implements.len());
        //Although we do not support xhp_attr_uses the pattern will still recurse to hint
        let _xhp_attr_uses = get_vec_len(&mut self.tree_stack, "Hint", p.xhp_attr_uses.len());
        let uses = get_vec_len(&mut self.tree_stack, "Hint", p.uses.len());
        let extends = get_vec_len(&mut self.tree_stack, "Hint", p.extends.len());
        let class_kind = self.string_stack.pop();
        let class = json!({
            "kind": "Class",
            "name": p.name.1,
            "span": visit_pos(p.span.clone(), self.show_pos),
            "class_kind": class_kind,
            "uses": uses,
            "extends": extends,
            "implements": implements,
            "where_constraints": where_constraints,
            "vars": vars,
            "methods": methods,
            "doc_comment": p.doc_comment.clone(),
        });
        self.tree_stack.push(class);
        Ok(())
    }

    fn visit_classish_kind(
        &mut self,
        _c: &mut Context,
        p: &'ast ast_defs::ClassishKind,
    ) -> Result<(), ()> {
        // println!("Classish Kind");
        let _ = p.recurse(_c, self.object());
        let kind = match p {
            ast_defs::ClassishKind::Cclass(a) =>{
                match a{
                    ast_defs::Abstraction::Concrete =>{
                        "ConcreteClass"
                    }
                    ast_defs::Abstraction::Abstract =>{
                        "AbstractClass"
                    }
                }
            }
            ast_defs::ClassishKind::Cinterface =>{
                "Interface"
            }
            ast_defs::ClassishKind::Ctrait =>{
                "Trait"
            }
            ast_defs::ClassishKind::Cenum => {
                "Enum"
            }
            ast_defs::ClassishKind::CenumClass(a) =>{
                match a{
                    ast_defs::Abstraction::Concrete =>{
                        "ConcreteEnumClass"
                    }
                    ast_defs::Abstraction::Abstract =>{
                        "AbstractEnumClass"
                    }
                }
            }
        };
        self.string_stack.push(kind.to_string());
        Ok(())
    }
    // Skip
    fn visit_collection_targ(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::CollectionTarg<()>,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }

    fn visit_constraint_kind(
        &mut self,
        _c: &mut Context,
        p: &'ast ast_defs::ConstraintKind,
    ) -> Result<(), ()> {
        let _ = p.recurse(_c, self.object());
        let constraint_kind = match p {
            ast_defs::ConstraintKind::ConstraintAs => "Constraint_as",
            ast_defs::ConstraintKind::ConstraintEq => "Constraint_eq",
            ast_defs::ConstraintKind::ConstraintSuper => "Constraint_super",
        };
        self.string_stack.push(constraint_kind.to_string());
        Ok(())
    }
    // PLEASE IMPLEMENT EVENTUALLY
    fn visit_contexts(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::Contexts,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Skip
    fn visit_ctx_refinement(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::CtxRefinement,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    // Skip 
    fn visit_ctx_refinement_bounds(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::CtxRefinementBounds,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }

    // Used
    fn visit_def(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Def<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(_c, self.object())
    }
    fn visit_default_case(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::DefaultCase<(), ()>,
    ) -> Result<(), ()> {
        // println!("Default Case");
        let _ = p.recurse(_c, self.object());
        let dc = json!({
            "kind": "DefaultCase",
            "block": self.tree_stack.pop(),
        });
        self.tree_stack.push(dc);
        Ok(())
    }
    //Ignored CaptureLid
    fn visit_efun(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Efun<(), ()>,
    ) -> Result<(), ()> {
        // println!("Expanded Lambda");
        let _ = p.recurse(_c, self.object());
        let ef = json!({
            "kind": "EfunLambda",
            "fun": self.tree_stack.pop(),
        });
        self.tree_stack.push(ef);
        Ok(())   
    }
    //Skip
    fn visit_emit_id(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::EmitId,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    // PLEASE IMPLEMENT EVENTUALLY
    fn visit_enum_(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::Enum_,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    // Used
    fn visit_expr(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Expr<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(_c, self.object())
    }

    fn visit_expr_(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Expr_<(), ()>,
    ) -> Result<(), ()> {
        // println!("Expr");
        let _ = p.recurse(_c, self.object());
        match p {
            aast_defs::Expr_::Null => {
                let expr = json!({
                    "kind": "Expr",
                    "type": "Null",
                });
                self.tree_stack.push(expr);
                
            }
            aast_defs::Expr_::True => {
                let expr = json!({
                    "kind": "Expr",
                    "type": "True",
                });
                self.tree_stack.push(expr);  
            }
            aast_defs::Expr_::False => {
                let expr = json!({
                    "kind": "Expr",
                    "type": "False",
                });
                self.tree_stack.push(expr);  
            }
            //Need Input
            aast_defs::Expr_::Shape(_) => {
                let expr = json!({
                    "kind": "Expr",
                    "type": "Shape",
                });
                self.tree_stack.push(expr);  
            }
            aast_defs::Expr_::ValCollection(b) => {
                let exprs = get_vec_len(&mut self.tree_stack, "Expr", b.2.len());
                let expr = json!({
                    "kind": "Expr",
                    "type": "ValCollection",
                    "VcKind": self.string_stack.pop(),
                    "expr": exprs,

                });
                self.tree_stack.push(expr);  
            }
            aast_defs::Expr_::KeyValCollection(b) => {
                let exprs = get_vec_len(&mut self.tree_stack, "Field", b.2.len());
                let expr = json!({
                    "kind": "Expr",
                    "type": "KeyValCollection",
                    "name": self.string_stack.pop(),
                    "expr": exprs,

                }); 
                self.tree_stack.push(expr);  
            }
            aast_defs::Expr_::This => {
                let expr = json!({
                    "kind": "Expr",
                    "type": "This",
                });
                self.tree_stack.push(expr);  
            }
            aast_defs::Expr_::Omitted => {
                let expr = json!({
                    "kind": "Expr",
                    "type": "Omitted",
                });
                self.tree_stack.push(expr);  
            }
            aast_defs::Expr_::Invalid(_) => {
                let expr = json!({
                    "kind": "Expr",
                    "type": "Invalid",
                    "expr": self.tree_stack.pop(),
                });
                self.tree_stack.push(expr);  
            }
            aast_defs::Expr_::Id(a) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "Id",
                    "expr": a.1.clone(),
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Lvar(a) => {
                let expr = json!({
                    "kind": "Expr",
                    "type": "LocalVar",
                    "expr": *a.1.1,
                });
                self.tree_stack.push(expr);  
            }
            aast_defs::Expr_::Dollardollar(_) => {
                let expr = json!({
                    "kind": "Expr",
                    "type": "Shape",

                });
                self.tree_stack.push(expr);  
            }
            aast_defs::Expr_::Clone(_) => {
                let expr = json!({
                    "kind": "Expr",
                    "type": "Clone",
                    "expr": self.tree_stack.pop(),
                });
                self.tree_stack.push(expr);  
            }
            aast_defs::Expr_::ArrayGet(b) => {
                let mut expr2 = Value::Null;
                if b.1.is_some(){
                    expr2 = self.tree_stack.pop().unwrap();
                }
                let expr1 = self.tree_stack.pop();
                let expr = json!({
                    "kind": "Expr",
                    "type": "ArrayGet",
                    "expr": [expr1, expr2],
                });
                self.tree_stack.push(expr);
            }
            aast_defs::Expr_::ObjGet(_)=>{
                let expr2 = self.tree_stack.pop();
                let expr1 = self.tree_stack.pop();
                let expr = json!({
                    "kind": "Expr",
                    "type": "ObjGet",
                    "expr": [expr1, expr2],
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::ClassGet(_)=>{
                let class_id = self.tree_stack.pop();
                let expr = json!({
                    "kind": "Expr",
                    "type": "ClassGet",
                    "expr": class_id,
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::ClassConst(a)=>{
                let class_id = self.tree_stack.pop();
                let expr = json!({
                    "kind": "Expr",
                    "type": "ClassConst",
                    "expr": [class_id, a.1.clone()],
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Call(_)=>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "Call",
                    "expr": self.tree_stack.pop(),
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::FunctionPointer(a)=>{
                let targs = get_vec_len(&mut self.tree_stack, "Targ", a.1.len());
                let expr = json!({
                    "kind": "Expr",
                    "type": "FunctionPointer",
                    "expr": targs,
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Int(a) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "Int",
                    "expr": a,
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Float(a) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "Float",
                    "expr": a,
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::String(a) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "String",
                    "expr": a.to_string(),
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::String2(a) =>{
                let exprs = get_vec_len(&mut self.tree_stack, "Expr", a.len());
                let expr = json!({
                    "kind": "Expr",
                    "type": "String2",
                    "expr": exprs,
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::PrefixedString(a) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "PrefixedString",
                    "expr": [a.0, self.tree_stack.pop()],
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Yield(_) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "Yield",
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Await(_) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "Await",
                    "expr": self.tree_stack.pop(),
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::ReadonlyExpr(_) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "ReadonlyExpr",
                    "expr": self.tree_stack.pop(),
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Tuple(a) =>{
                let exprs = get_vec_len(&mut self.tree_stack, "Expr", a.len());
                let expr = json!({
                    "kind": "Expr",
                    "type": "Tuple",
                    "expr": exprs,
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::List(a) =>{
                let exprs = get_vec_len(&mut self.tree_stack, "Expr", a.len());
                let expr = json!({
                    "kind": "Expr",
                    "type": "List",
                    "expr": exprs,
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Cast(_) =>{
                let hint = self.tree_stack.pop();
                let ex = self.tree_stack.pop();
                let expr = json!({
                    "kind": "Expr",
                    "type": "Cast",
                    "expr": [hint, ex],
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Unop(_) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "Unop",
                    "name:": self.string_stack.pop(),
                    "expr": self.tree_stack.pop(),
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Binop(_)=>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "Binop",
                    "expr": self.tree_stack.pop(),
                });
                self.tree_stack.push(expr); 

            }
            aast_defs::Expr_::Pipe(a) =>{
                let expr2 = self.tree_stack.pop();
                let expr1 = self.tree_stack.pop();
                let expr = json!({
                    "kind": "Expr",
                    "type": "Pipe",
                    "expr": [a.0.1.clone(),expr1, expr2],
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Eif(a) =>{
                let expr3 = self.tree_stack.pop();
                let mut expr2 = Value::Null;
                if a.1.is_some(){
                    expr2 = self.tree_stack.pop().unwrap();
                }
                let expr1 = self.tree_stack.pop();
                if expr2 != Value::Null{
                    let expr = json!({
                        "kind": "Expr",
                        "type": "Eif",
                        "expr": [expr1, expr2, expr3],
                    });
                    self.tree_stack.push(expr); 
                }
                else {
                    let expr = json!({
                        "kind": "Expr",
                        "type": "Eif",
                        "expr": [expr1, expr3],
                    });
                    self.tree_stack.push(expr); 
                }
            }
            aast_defs::Expr_::Is(_) =>{
                let hint = self.tree_stack.pop();
                let ex = self.tree_stack.pop();
                let expr = json!({
                    "kind": "Expr",
                    "type": "Is",
                    "expr": [hint, ex],
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::As(_) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "As",
                    "expr": self.tree_stack.pop(),
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Upcast(_) =>{
                let hint = self.tree_stack.pop();
                let ex = self.tree_stack.pop();
                let expr = json!({
                    "kind": "Expr",
                    "type": "Upcast",
                    "expr": [hint, ex],
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::New(a) =>{
                if a.3.is_some(){
                    let opt_expr = self.tree_stack.pop();
                    let vec_exprs = get_vec_len(&mut self.tree_stack, "Expr", a.2.len());
                    let class_id = self.tree_stack.pop();
                    let expr = json!({
                        "kind": "Expr",
                        "classId": class_id,
                        "exprs": vec_exprs,
                        "opt": opt_expr,
                    });
                    self.tree_stack.push(expr); 
                }
                else{
                    let vec_exprs = get_vec_len(&mut self.tree_stack, "Expr", a.2.len());
                    // let vec_targs = get_vec_len(&mut self.tree_stack, "Targ", a.1.len());
                    let expr = json!({
                        "kind": "Expr",
                        "classId": self.tree_stack.pop(),
                        // "targs": vec_targs,
                        "exprs": vec_exprs,
                    });
                    self.tree_stack.push(expr); 
                }
            }
            aast_defs::Expr_::Efun(_) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "Efun",
                    "expr": self.tree_stack.pop(),
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Lfun(_) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "Lfun",
                    "expr": self.tree_stack.pop(),
                });
                self.tree_stack.push(expr); 
            }
            //Do Not Fully Support
            aast_defs::Expr_::Xml(a) =>{
                let exprs = get_vec_len(&mut self.tree_stack, "Expr", a.2.len());
                let expr = json!({
                    "kind": "Expr",
                    "type": "String",
                    "name": a.0.1.clone(),
                    "expr": exprs,
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Import(_) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "String",
                    "name:": self.string_stack.pop(),
                    "expr": self.tree_stack.pop(),
                });
                self.tree_stack.push(expr); 
            }
            //Do Not Support Afield
            aast_defs::Expr_::Collection(a) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "String",
                    "name": a.0.1.clone(),
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::ExpressionTree(_) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "String",
                    "expr": self.tree_stack.pop(),
                });
                self.tree_stack.push(expr); 
            }
            //Removed Info
            aast_defs::Expr_::Lplaceholder(_) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "Lplaceholder",
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::MethodCaller(a) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "MethodCaller",
                    "expr": [a.0.1.clone(), a.1.1.clone()],
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Pair(_) =>{
                let e2 = self.tree_stack.pop();
                let e1 = self.tree_stack.pop();
                let expr = json!({
                    "kind": "Expr",
                    "type": "Pair",
                    "expr": [e1, e2],
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::ETSplice(_) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "ETSplice",
                    "expr": self.tree_stack.pop(),
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::EnumClassLabel(a) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "EnumClassLabel",
                    "expr": a.1.clone(),
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Hole(_) =>{
                let hs = self.string_stack.pop();
                let expr = self.tree_stack.pop();
                let expr = json!({
                    "kind": "Expr",
                    "type": "Hole",
                    "expr": [expr, hs],
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Package(a) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "Package",
                    "expr": a.1.clone(),
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Nameof(_) =>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "Nameof",
                    "expr": self.tree_stack.pop(),
                });
                self.tree_stack.push(expr); 
            }
        }
        Ok(())
    }
    fn visit_expression_tree(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::ExpressionTree<(), ()>,
    ) -> Result<(), ()> {
        // println!("ExpressionTree");
        let _ = p.recurse(_c, self.object());
        let runtime_expr = self.tree_stack.pop();
        let function_pointers = get_vec_len(&mut self.tree_stack, "Stmt_", p.function_pointers.len());
        let splices = get_vec_len(&mut self.tree_stack, "Stmt_", p.splices.len());
        let expr = json!({
            "kind": "ExpressionTree",
            "class": p.class.1.clone(),
            "splices": splices,
            "function_pointers": function_pointers,
            "runtime_expr": runtime_expr,
        });
        self.tree_stack.push(expr);
        Ok(())
    }
    fn visit_field(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Field<(), ()>,
    ) -> Result<(), ()> {
        // println!("Fields");
        let _ = p.recurse(_c, self.object());
        let expr2 = self.string_stack.pop();
        let expr1 = self.string_stack.pop();
        let field = json!({
            "kind": "Field",
            "expr": [expr1, expr2],
        });
        self.tree_stack.push(field);
        Ok(())
   
    }
    // PLEASE IMPLEMENT EVENTUALLY
    fn visit_file_attribute(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::FileAttribute<(), ()>,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    fn visit_finally_block(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::FinallyBlock<(), ()>,
    ) -> Result<(), ()> {
        // println!("Finally Block");
        let _ = p.recurse(_c, self.object());
        let stmts = get_vec_len(&mut self.tree_stack, "Stmt", p.0.len());
        let fb = json!({
            "kind": "FinallyBlock",
            "stmts": stmts,
        });
        self.tree_stack.push(fb);
        Ok(())
    }
   
   fn visit_fun_def(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::FunDef<(), ()>,
    ) -> Result<(), ()> {
        // println!("FunDef");
        let _ = p.recurse(_c, self.object());
        let fd = json!({
            "kind": "FunDef",
            "name": p.name.1.clone(),
            "child": self.tree_stack.pop(),
		});
        self.tree_stack.push(fd);
        Ok(())
    }
    // PLEASE IMPLEMENT EVENTUALLY
    fn visit_fun_kind(
        &mut self,
        _c: &mut Context,
        _p: &'ast ast_defs::FunKind,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
  
    fn visit_fun_param(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::FunParam<(), ()>,
    ) -> Result<(), ()> {
        // println!("FunParam");
        let _ = p.recurse(_c, self.object());

        let mut visibility = "Unknown".to_string();
        let mut expr = Value::Null;
        if p.expr.is_some(){ expr = self.tree_stack.pop().unwrap()};
        let span = visit_pos(p.pos.clone(), self.show_pos);
        if p.visibility.is_some(){ visibility = self.string_stack.pop().unwrap()};
        let type_hint = self.tree_stack.pop();

		let fp = json!({
            "kind": "FunParam",
            "name": p.name.clone(),
            "type_hint": type_hint,
            "pos": span,
            "expr": expr,
            "visibility": visibility,
        });
		self.tree_stack.push(fp);
		Ok(())
    }
    
    fn visit_fun_(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Fun_<(), ()>,
    ) -> Result<(), ()> {
        // println!("Fun_");
        let _ = p.recurse(_c, self.object());

        let body = self.tree_stack.pop();
        let params = get_vec_len(&mut self.tree_stack,"FunParam", p.params.len());
        let ret = self.tree_stack.pop();

        let f = json!({
                "kind": "Fun",
                "span": visit_pos(p.span.clone(), self.show_pos),
                "ret": ret,
                "params": params,
                "body": body,
                "doc_comment": p.doc_comment.clone(),
            });
        self.tree_stack.push(f);
        Ok(())
    }

    fn visit_func_body(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::FuncBody<(), ()>,
    ) -> Result<(), ()> {
        // println!("FuncBody");
        let _ = p.recurse(_c, self.object());

        let fb = json!({
            "kind": "FuncBody",
            "body": self.tree_stack.pop(),
        });
        self.tree_stack.push(fb);
        Ok(())
    }
    // Skip
    fn visit_function_ptr_id(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::FunctionPtrId<(), ()>,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    // Skip
    fn visit_gconst(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::Gconst<(), ()>,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    // Skip
    fn visit_hf_param_info(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::HfParamInfo,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    // Used
    fn visit_hint(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Hint,
    ) -> Result<(), ()> {
        p.recurse(_c, self.object())
    }
    fn visit_hint_fun(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::HintFun,
    ) -> Result<(), ()> {
        let _ = p.recurse(_c, self.object());
  
        let ret_ty = self.tree_stack.pop();
        let hints = get_vec_len(&mut self.tree_stack, "Hint", p.param_tys.len());
        let hint_fun = json!({
            "kind": "HintFun",
            "param_tys": hints,
            "return_ty": ret_ty,
        });
        self.tree_stack.push(hint_fun);
        Ok(())
    }
    fn visit_hint_(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Hint_,
    ) -> Result<(), ()> {
        // println!("Hint");
        let _ = p.recurse(_c, self.object());
        let ht = match p {
            aast::Hint_::Hprim(_)=>{
                json!({
                    "kind": "Hint",
                    "type": "Prim",
                    "hint": self.string_stack.pop(),//Tprim
                })
            },
            aast::Hint_::Happly(n, v) =>{
                let hints = get_vec_len(&mut self.tree_stack, "Hint", v.len());
                json!({
                    "kind": "Hint",
                    "type": "Apply",
                    "name": n.1.clone(),
                    "hints": hints,
                })
            },
            aast::Hint_::Hoption(_) =>{
                json!({
                    "kind": "Hint",
                    "type": "Option",
                    "hint": self.tree_stack.pop(),
                })
            },
            aast::Hint_::Hlike(_) =>{
                json!({
                    "kind": "Hint",
                    "type": "Like",
                    "hint": self.tree_stack.pop(),
                })
            },
            aast::Hint_::Hfun(_) =>{
                json!({
                    "kind": "Hint",
                    "type": "Fun",
                    "hint": self.tree_stack.pop()
                })
            },
            aast::Hint_::Htuple(h) =>{
                let hints = get_vec_len(&mut self.tree_stack, "Hint", h.len());
                json!({
                    "kind": "Hint",
                    "type": "Tuple",
                    "hints": hints,
                })
            },
            aast::Hint_::HclassArgs(_) =>{
                json!({
                    "kind": "Hint",
                    "type": "ClassArgs",
                    "hint": self.tree_stack.pop(),
                })
            },
            aast::Hint_::Hshape(_) =>{

                json!({
                    "kind": "Hint",
                    "type": "ClassArgs",
                    "hint": self.tree_stack.pop(),
                })
            },
            aast::Hint_::Haccess(_, _) =>{
                json!({
                    "kind": "Hint",
                    "type": "Access",
                    "hint": self.tree_stack.pop(),
                })
            },
            aast::Hint_::Hsoft(_) =>{
                json!({
                    "kind": "Hint",
                    "type": "Soft",
                    "hint": self.tree_stack.pop(),
                })
            },
            aast::Hint_::Hrefinement(_, _) =>{
                json!({
                    "kind": "Hint",
                    "type": "Refinement",
                    "hint": self.tree_stack.pop(),
                })
            },
            aast::Hint_::Hmixed =>{
                json!({
                    "kind": "Hint",
                    "type": "Mixed",
                })
            },
            aast::Hint_::Hwildcard =>{
                json!({
                    "kind": "Hint",
                    "type": "Wildcard",
                })
            },
            aast::Hint_::Hnonnull =>{
                json!({
                    "kind": "Hint",
                    "type": "NonNull",
                })
            },
            aast::Hint_::Habstr(s, v) =>{
                let hints = get_vec_len(&mut self.tree_stack, "Hint", v.len());       
                json!({
                    "kind": "Hint",
                    "type": "Abstr",
                    "name": s.clone(),
                    "hints": hints,
                })
            },
            aast::Hint_::HvecOrDict(o, _) =>{
                if o.is_some(){
                    let o = self.tree_stack.pop();
                    let hint = self.tree_stack.pop();
                    json!({
                        "kind": "Hint",
                        "type": "VecOrDict",
                        "hint": [o, hint],
                    })
                }
                else{
                    json!({
                        "kind": "Hint",
                        "type": "VecOrDict",
                        "hint": self.tree_stack.pop(),
                    })
                }
            },
            aast::Hint_::Hthis =>{
                json!({
                    "kind": "Hint",
                    "type": "This",
                })
            },
            aast::Hint_::Hdynamic =>{
                json!({
                    "kind": "Hint",
                    "type": "Dynamic",
                })
            },
            aast::Hint_::Hnothing =>{
                json!({
                    "kind": "Hint",
                    "type": "Nothing",
                })
            },
            aast::Hint_::Hunion(h) =>{
                let hints = get_vec_len(&mut self.tree_stack, "Hint", h.len());       
                json!({
                    "kind": "Hint",
                    "type": "Union",
                    "hint": hints,
                })
            },
            aast::Hint_::Hintersection(h) =>{
                let hints = get_vec_len(&mut self.tree_stack, "Hint", h.len());       
                json!({
                    "kind": "Hint",
                    "type": "Intersection",
                    "hint": hints,
                })
            },
            aast::Hint_::HfunContext(s) =>{
                json!({
                    "kind": "Hint",
                    "type": "FunContext",
                    "hint": s,
                })
            },
            aast::Hint_::Hvar(s) =>{
                json!({
                    "kind": "Hint",
                    "type": "Var",
                    "hint": s,
                })
            },
        };
        self.tree_stack.push(ht);
        Ok(())
    }

    fn visit_hole_source(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::HoleSource,
    ) -> Result<(), ()> {
        // println!("Hole Source");
        // let _ = p.recurse(_c, self.object());
        let h = match p {
            aast::HoleSource::Typing => "Typing",
            aast::HoleSource::UnsafeCast(_) => "UnsafeCast", 
            aast::HoleSource::UnsafeNonnullCast => "UnsafeNonnullCast",
            aast::HoleSource::EnforcedCast(_) => "EnforcedCast", 
        };
        self.string_stack.push(h.to_string());
        Ok(())
    }
    // Skip
    fn visit_id(
        &mut self,
        _c: &mut Context,
        _p: &'ast ast_defs::Id,
    ) -> Result<(), ()> {
        // let _ = p.recurse(_c, self.object());
        Ok(())
    }
    fn visit_import_flavor(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::ImportFlavor,
    ) -> Result<(), ()> {
        let _ = p.recurse(_c, self.object());
        let fv = match p{
            aast::ImportFlavor::Include => "Include",
            aast::ImportFlavor::Require => "Require",
            aast::ImportFlavor::IncludeOnce => "IncludeOnce",
            aast::ImportFlavor::RequireOnce => "RequireOnce",
        };
        self.string_stack.push(fv.to_string());
        Ok(())
    }
    fn visit_kvc_kind(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::KvcKind,
    ) -> Result<(), ()> {
        // println!("KvcKind");
        let _ = p.recurse(_c, self.object());
        let kvc = match p{
            aast::KvcKind::Map => "Map",
            aast::KvcKind::ImmMap => "ImmMap",
            aast::KvcKind::Dict => "Dict",
        };
        self.string_stack.push(kvc.to_string());
        Ok(())
    }
    // Skip
    fn visit_lid(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::Lid,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Skip
    fn visit_md_name_kind(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::MdNameKind,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    
    fn visit_method_(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Method_<(), ()>,
    ) -> Result<(), ()> {
        // println!("Method");
        let _ = p.recurse(_c, self.object());

        let ret = self.tree_stack.pop();
        let body = self.tree_stack.pop();
        let params = get_vec_len(&mut self.tree_stack,"FunParam", p.params.len());
        let where_constraints = get_vec_len(&mut self.tree_stack,"WhereConstraintHint", p.where_constraints.len());
        let m = json!({
            "kind": "Method",
            "name": p.name.1.clone(),
            "span": visit_pos(p.name.0.clone(), self.show_pos),
            "where_constraints": where_constraints,
            "params": params,
            "visibility": self.string_stack.pop(),
            "is_final": p.final_.clone(),
            "is_abstract": p.abstract_.clone(),
            "is_static": p.static_.clone(),
            "body": body,
            "ret": ret,
            "doc_comment": p.doc_comment.clone(),
            
        });
        self.tree_stack.push(m);
        Ok(())
    }
    // Skip
    fn visit_module_def(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::ModuleDef<(), ()>,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    // Skip
    fn visit_nast_shape_info(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::NastShapeInfo,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Skip
    fn visit_ns_kind(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::NsKind,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Skip
    fn visit_og_null_flavor(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::OgNullFlavor,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Skip
    fn visit_optional_kind(
        &mut self,
        _c: &mut Context,
        _p: &'ast ast_defs::OptionalKind,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    // PLEASE IMPLEMENT EVENTUALLY
    fn visit_param_kind(
        &mut self,
        _c: &mut Context,
        _p: &'ast ast_defs::ParamKind,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Skip
    fn visit_pat_refinement(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::PatRefinement,
    ) -> Result<(), ()> {
        p.recurse(_c, self.object())
    }
    //Skip
    fn visit_pat_var(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::PatVar,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Skip
    fn visit_pattern(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::Pattern,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }

    fn visit_program(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Program<(), ()>,
    ) -> Result<(), ()> {
        // println!("Program");
        let _ = p.recurse(_c, self.object());
        let mut defs = VecDeque::<Value>::new();
        while let Some(last) = self.tree_stack.pop() {
            defs.push_front(last);
        }
        let obj = json!({
            "kind": "Program",
            "definitions": defs,
        });

        self.tree = serde_json::to_string_pretty(&obj).unwrap();
        // println!("{}", serde_json::to_string_pretty(&obj).unwrap());
        Ok(())
    }
    // Skip
    fn visit_prop_or_method(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::PropOrMethod,
    ) -> Result<(), ()> {
        p.recurse(_c, self.object())
    }
    // Skip
    fn visit_readonly_kind(
        &mut self,
        _c: &mut Context,
        p: &'ast ast_defs::ReadonlyKind,
    ) -> Result<(), ()> {
        p.recurse(_c, self.object())
    }
    // Skip
    fn visit_refinement(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::Refinement,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Skip
    fn visit_reify_kind(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::ReifyKind,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Skip
    fn visit_require_kind(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::RequireKind,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Skip
    fn visit_shape_field_info(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::ShapeFieldInfo,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Skip
    fn visit_shape_field_name(
        &mut self,
        _c: &mut Context,
        _p: &'ast ast_defs::ShapeFieldName,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Used
    fn visit_stmt(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Stmt<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(_c, self.object())
    }
    fn visit_stmt_match(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::StmtMatch<(), ()>,
    ) -> Result<(), ()> {
        // println!("Stmt Match");
        let _ = p.recurse(_c, self.object());
        let arms = get_vec_len(&mut self.tree_stack, "StmtMatchArm", p.arms.len());
        let expr = self.tree_stack.pop();
        let stmt = json!({
            "kind": "StmtMatch",
            "expr": expr,
            "arms": arms,
        });
        self.tree_stack.push(stmt);
        Ok(())
    }
    //Ignored Pattern
    fn visit_stmt_match_arm(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::StmtMatchArm<(), ()>,
    ) -> Result<(), ()> {
        // println!("Stmt Match Arm");
        let _ = p.recurse(_c, self.object());
        let stmt = json!({
            "kind": "StmtMatchArm",
            "body": self.tree_stack.pop(),
        });
        self.tree_stack.push(stmt);
        Ok(())
    }
    fn visit_stmt_(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Stmt_<(), ()>,
    ) -> Result<(), ()> {
        // println!("Stmt");
        let _ = p.recurse(_c, self.object());
        let stmt = match p{
            aast::Stmt_::Noop => {
                json!({
                    "kind": "Stmt",
                    "type": "Noop",
                })
            }
            aast::Stmt_::Fallthrough => {
                json!({
                    "kind": "Stmt",
                    "type": "Fallthrough",
                })
            }
            aast::Stmt_::Expr(_) => {
                json!({
                    "kind": "Stmt",
                    "type": "Expr",
                    "stmt": self.tree_stack.pop(),
                })
            }
            aast::Stmt_::Break => {
                json!({
                    "kind": "Stmt",
                    "type": "Break",
                })
            }
            aast::Stmt_::Continue => {
                json!({
                    "kind": "Stmt",
                    "type": "Continue",
                })
            }
            aast::Stmt_::Throw(_) => {
                json!({
                    "kind": "Stmt",
                    "type": "Throw",
                    "stmt": self.tree_stack.pop(),
                })
            }
            aast::Stmt_::Return(expr) => {
                if (*expr).is_none(){
                    json!({
                        "kind": "Stmt",
                        "type": "Return",
                    })
                }
                else {
                    json!({
                        "kind": "Stmt",
                        "type": "Return",
                        "stmt": self.tree_stack.pop()
                    })
                }
            }
            aast::Stmt_::YieldBreak => {
                json!({
                    "kind": "Stmt",
                    "type": "YieldBreak",
                })
            }
            aast::Stmt_::Awaitall(a) => {
                let block = self.tree_stack.pop();
                let exprs = get_vec_len(&mut self.tree_stack, "Expr", a.0.len());
                json!({
                    "kind": "Stmt",
                    "type": "Awaitall",
                    "stmt": [block, exprs],
                })
            }
            aast::Stmt_::Concurrent(_) => {
                json!({
                    "kind": "Stmt",
                    "type": "Concurrent",
                    "stmt": self.tree_stack.pop(),
                })
            }
            aast::Stmt_::If(_) => {
                let b2 = self.tree_stack.pop();
                let b1 = self.tree_stack.pop();
                let expr = self.tree_stack.pop();
                json!({
                    "kind": "Stmt",
                    "type": "If",
                    "stmt": [expr, b1, b2],
                })
                
            }
            aast::Stmt_::Do(_) => {
                let expr = self.tree_stack.pop();
                let b = self.tree_stack.pop();
                json!({
                    "kind": "Stmt",
                    "type": "Do",
                    "stmt": [b, expr],
                })
            }
            aast::Stmt_::While(_) => {
                let b = self.tree_stack.pop();
                let expr = self.tree_stack.pop();
                json!({
                    "kind": "Stmt",
                    "type": "While",
                    "stmt": [expr, b],
                })
            }
            aast::Stmt_::Using(_) => {
                json!({
                    "kind": "Stmt",
                    "type": "Using",
                    "stmt": self.tree_stack.pop(),
                })
            }
            aast::Stmt_::For(f) => {
                let b = self.tree_stack.pop();
                let exprs2 = get_vec_len(&mut self.tree_stack, "Expr", f.2.len());
                if f.1.is_some(){
                    let opt_expr = self.tree_stack.pop();
                    let exprs1 = get_vec_len(&mut self.tree_stack, "Expr", f.0.len());
                    json!({
                        "kind": "Stmt",
                        "type": "For",
                        "stmt": [exprs1, opt_expr, exprs2, b],
                    })
                }
                else{
                    let exprs1 = get_vec_len(&mut self.tree_stack, "Expr", f.0.len());
                    json!({
                        "kind": "Stmt",
                        "type": "For",
                        "stmt": [exprs1, exprs2, b],
                    })
                }
            }
            aast::Stmt_::Switch(s) => {
                let mut dc = Value::Null;
                if s.2.is_some(){
                    dc = self.tree_stack.pop().unwrap();
                }
                let cases = get_vec_len(&mut self.tree_stack, "Case", s.1.len());
                let expr = self.tree_stack.pop();
                if dc != Value::Null{
                    json!({
                        "kind": "Stmt",
                        "type": "Switch",
                        "stmt": [expr, cases, dc],
                    })
                }
                else{
                    json!({
                        "kind": "Stmt",
                        "type": "Switch",
                        "stmt": [expr, cases],
                    })
                }
            }
            aast::Stmt_::Match(_) => {
                json!({
                    "kind": "Stmt",
                    "type": "Match",
                    "stmt": self.tree_stack.pop(),
                })
            }
            aast::Stmt_::Foreach(_) => {
                let block = self.tree_stack.pop();
                let as_expr = self.tree_stack.pop();
                let expr = self.tree_stack.pop();
                json!({
                    "kind": "Stmt",
                    "type": "Try",
                    "stmt": [block, as_expr, expr],
                })
            }
            aast::Stmt_::Try(ts) => {
                let finally_block = self.tree_stack.pop();
                let catchs = get_vec_len(&mut self.tree_stack, "Catch", ts.1.len());
                let block = self.tree_stack.pop();
                json!({
                    "kind": "Stmt",
                    "type": "Try",
                    "stmt": [block, catchs, finally_block],
                })
            }
            aast::Stmt_::DeclareLocal(dl) => {
                if dl.2.is_some(){
                    json!({
                        "kind": "Stmt",
                        "type": "DeclareLocal",
                        "stmt": self.tree_stack.pop()
                    })
                }
                else{
                    json!({
                        "kind": "Stmt",
                        "type": "DeclareLocal",
                    })
                }
            }
            aast::Stmt_::Block(_) => {
                json!({
                    "kind": "Stmt",
                    "type": "Block",
                    "stmt": self.tree_stack.pop(),
                })
            }
            aast::Stmt_::Markup(m) => {
                json!({
                    "kind": "Stmt",
                    "type": "Markup",
                    "stmt": m.1,
                })
            }
        };
        self.tree_stack.push(stmt);
        Ok(())
    }
    // PLEASE IMPLEMENT EVENTUALLY
    fn visit_targ(
        &mut self,
        __c: &mut Context,
        _p: &'ast aast::Targ<()>,
    ) -> Result<(), ()> {
        Ok(())
    }
    // PLEASE IMPLEMENT EVENTUALLY
    fn visit_tparam(
        &mut self,
        __c: &mut Context,
        _p: &'ast aast::Tparam<(), ()>,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    fn visit_tprim(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Tprim,
    ) -> Result<(), ()> {
        let _ = p.recurse(_c, self.object());
        let prim = match p {
            ast_defs::Tprim::Tnull => "Tnull",
            ast_defs::Tprim::Tvoid => "Tvoid",
            ast_defs::Tprim::Tint => "Tint",
            ast_defs::Tprim::Tbool => "Tbool",
            ast_defs::Tprim::Tfloat => "Tfloat",
            ast_defs::Tprim::Tstring => "Tstring",
            ast_defs::Tprim::Tresource => "Tresource",
            ast_defs::Tprim::Tnum => "Tnum",
            ast_defs::Tprim::Tarraykey => "Tarraykey",
            ast_defs::Tprim::Tnoreturn => "Tnoreturn",
        };
        self.string_stack.push(prim.to_string());
        Ok(())
    }

    fn visit_type_hint(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::TypeHint<()>,
    ) -> Result<(), ()> {
        //Type Hint can be optional
        if p.1.is_some(){
            let _ = p.recurse(_c, self.object());
        }
        else{
            self.tree_stack.push(json!("None"));
        }
        Ok(())
    }
    //Skip
    fn visit_type_refinement(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::TypeRefinement,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Skip
    fn visit_type_refinement_bounds(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::TypeRefinementBounds,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    // PLEASE IMPLEMENT EVENTUALLY
    fn visit_typedef(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::Typedef<(), ()>,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    // PLEASE IMPLEMENT EVENTUALLY
    fn visit_typedef_visibility(
        &mut self,
        __c: &mut Context,
        _p: &'ast aast::TypedefVisibility,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }

    fn visit_uop(
        &mut self,
        _c: &mut Context,
        p: &'ast ast_defs::Uop,
    ) -> Result<(), ()> {
        // println!("Uop");
        let uop = match p{
            ast_defs::Uop::Utild => "Utild",
            ast_defs::Uop::Unot => "Unot",
            ast_defs::Uop::Uplus => "Uplus",
            ast_defs::Uop::Uminus => "Uminus",
            ast_defs::Uop::Uincr => "Uincr",
            ast_defs::Uop::Udecr => "Udecr",
            ast_defs::Uop::Upincr => "Upincr",
            ast_defs::Uop::Updecr => "Updecr",
            ast_defs::Uop::Usilence => "Usilence",
        };
        self.string_stack.push(uop.to_string());
        Ok(())
    }
    //Skip
    fn visit_user_attribute(
        &mut self,
        _c: &mut Context,
        _p: &'ast aast::UserAttribute<(), ()>,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Skip
    fn visit_user_attributes(
        &mut self,
        __c: &mut Context,
        _p: &'ast aast::UserAttributes<(), ()>,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }

    fn visit_using_stmt(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::UsingStmt<(), ()>,
    ) -> Result<(), ()> {
        // println!("Using Stmt");
        let _ = p.recurse(_c, self.object());
        let block = self.tree_stack.pop();
        let exprs = get_vec_len(&mut self.tree_stack, "Expr", p.exprs.1.len());
        let us = json!({
            "kind": "UsingStmt",
            "exprs": exprs,
            "block": block,
        });
        self.tree_stack.push(us);
        Ok(())
    }
    //Skip
    fn visit_variance(
        &mut self,
        __c: &mut Context,
        _p: &'ast ast_defs::Variance,
    ) -> Result<(), ()> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    fn visit_vc_kind(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::VcKind,
    ) -> Result<(), ()> {
        // println!("VcKind");
        let _ = p.recurse(_c, self.object());
        let vc = match p {
            aast::VcKind::Vector => "Vector",
            aast::VcKind::ImmVector => "ImmVector",
            aast::VcKind::Vec => "Vec",
            aast::VcKind::Set => "Set",
            aast::VcKind::ImmSet => "ImmSet",
            aast::VcKind::Keyset => "Keyset",
        };
        self.string_stack.push(vc.to_string());
        Ok(())
    }

    fn visit_visibility(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::Visibility,
    ) -> Result<(), ()> {
        // println!("Visibility");
        let _ = p.recurse(_c, self.object());
        let visibility = match p {
            ast_defs::Visibility::Private => "Private",
            ast_defs::Visibility::Public => "Public",
            ast_defs::Visibility::Protected => "Protected",
            ast_defs::Visibility::Internal => "Internal",
        };
        self.string_stack.push(visibility.to_string());
        Ok(())
    }
    
    fn visit_where_constraint_hint(
        &mut self,
        _c: &mut Context,
        p: &'ast aast::WhereConstraintHint,
    ) -> Result<(), ()> {
        let _ = p.recurse(_c, self.object());
        let h2 = self.tree_stack.pop();
        let constraint = self.string_stack.pop();
        let h1 = self.tree_stack.pop();
        let where_constraint = json!({
            "kind": "WhereConstraintHint",
            "hints": [h1, h2],
            "constraint_kind": constraint,
        });
        self.tree_stack.push(where_constraint);
        Ok(())
    }
    
    //Do Not Support
    fn visit_xhp_attr(
        &mut self,
        _c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        _p: &'ast aast::XhpAttr<<Self::Params as oxidized::aast_visitor::Params>::Ex, <Self::Params as oxidized::aast_visitor::Params>::En>,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Do Not Support
    fn visit_xhp_attr_info(
        &mut self,
        _c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        _p: &'ast aast::XhpAttrInfo,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Do Not Support
    fn visit_xhp_attr_tag(
        &mut self,
        _c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        _p: &'ast aast::XhpAttrTag,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Do Not Support
    fn visit_xhp_attribute(
        &mut self,
        _c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        _p: &'ast aast::XhpAttribute<<Self::Params as oxidized::aast_visitor::Params>::Ex, <Self::Params as oxidized::aast_visitor::Params>::En>,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Do Not Support
    fn visit_xhp_child(
        &mut self,
        _c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        _p: &'ast aast::XhpChild,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        // p.recurse(_c, self.object())
        Ok(())
    }
    //Do Not Support
    fn visit_xhp_child_op(
        &mut self,
        _c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        _p: &'ast aast::XhpChildOp,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        // p.recurse(_c, self.object())
         Ok(())
    }
    //Do Not Support
    fn visit_xhp_enum_value(
        &mut self,
        _c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        _p: &'ast ast_defs::XhpEnumValue,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        // p.recurse(_c, self.object())
         Ok(())
    }
    //Do Not Support
    fn visit_xhp_simple(
        &mut self,
        _c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        _p: &'ast aast::XhpSimple<<Self::Params as oxidized::aast_visitor::Params>::Ex, <Self::Params as oxidized::aast_visitor::Params>::En>,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        // p.recurse(_c, self.object())
         Ok(())
    }
}
