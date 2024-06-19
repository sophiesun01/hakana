extern crate json_value_merge;

use std::collections::VecDeque;
use std::collections::HashSet;
use json_value_merge::Merge;

use oxidized::{
    aast,
    aast_visitor::{visit, AstParams, Node, Visitor},
    ast_defs,
    aast_defs,
};
use rustc_hash::FxHashMap;
use serde_json::{self, json};
use serde_json::Value;

pub(crate) struct Scanner {
    pub tree_stack: Vec<Value>,
    pub tree: String,
    pub string_stack: Vec<String>,
    // pub interner: &'a mut ThreadedInterner
}
pub(crate) struct Context {

}

fn get_vec(tree_stack: &mut Vec<Value>, kind: &str
    )-> VecDeque<Value> {

    let mut arr = VecDeque::<Value>::new();
    while let Some(last) = tree_stack.last() {
        if last["kind"] != kind {
            break;
        }
        arr.push_front(tree_stack.pop().unwrap());
    }
    arr
}

fn get_vec_len(tree_stack: &mut Vec<Value>, kind: &str, mut n: usize
    )-> VecDeque<Value> {

let mut arr = VecDeque::new();
while let Some(last) = tree_stack.last() {
    if last["kind"] != kind || n == 0 {
        if n > 0{
            println!("Something ain't right {}", kind);
        }
        break;
    }

    n -= 1;
    arr.push_front(tree_stack.pop().unwrap());
}
arr
}

// fn get_vec_string(tree_stack: &mut Vec<String>, kind: &str
// )-> VecDeque<String> {

// let mut arr = VecDeque::<String>::new();
// while let Some(last) = tree_stack.last() {
//     if last["kind"] != kind {
//         break;
//     }
//     arr.push_front(tree_stack.pop().unwrap());
// }
// arr
// }


impl <'ast>Visitor<'ast> for Scanner {
	type Params = AstParams<Context, ()>;
	fn object(&mut self) -> &mut dyn Visitor<'ast, Params = Self::Params> {
        self
	}

    // fn visit_ex(
    //     &mut self,
    //     c: &mut Context,
    //     p: &'ast ast_defs::Ex,
    // ) -> Result<(), ()> {
    //     Ok(())
    // }
    // fn visit_en(
    //     &mut self,
    //     c: &mut Context,
    //     p: &'ast ast_defs::En,
    // ) -> Result<(), ()> {
    //     Ok(())
    // }
    
    //Done - Implemented in Classish
    fn visit_abstraction(
        &mut self,
        c: &mut Context,
        p: &'ast ast_defs::Abstraction,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_afield(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Afield<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_as_expr(
        &mut self,
        c: &mut Context,
        p: &'ast aast::AsExpr<(), ()>,
    ) -> Result<(), ()> {
        println!("As Expr");
        let _ = p.recurse(c, self.object());
        let mut ae = Value::Null;
        match p{
            aast::AsExpr::AsV(e) =>{
                ae = json!({
                    "kind": "AsExpr",
                    "type": "AsV",
                    "asExpr": self.tree_stack.pop(),
                })
            }
            aast::AsExpr::AsKv(e1, e2) =>{
                let e2 = self.tree_stack.pop();
                let e1 = self.tree_stack.pop();
                ae = json!({
                    "kind": "AsExpr",
                    "type": "AsKv",
                    "asExpr": [e1, e2],
                })
            }
            aast::AsExpr::AwaitAsV(p, e) =>{
                ae = json!({
                    "kind": "AsExpr",
                    "type": "AwaitAsV",
                    "asExpr": self.tree_stack.pop(),
                })
            }
            aast::AsExpr::AwaitAsKv(_,_, _) =>{
                let e2 = self.tree_stack.pop();
                let e1 = self.tree_stack.pop();

                ae = json!({
                    "kind": "AsExpr",
                    "type": "AwaitAsKv",
                    "asExpr": [e1, e2],
                })
            }
        }
        self.tree_stack.push(ae);
        Ok(())
    }
    fn visit_as_(
        &mut self,
        c: &mut Context,
        p: &'ast aast::As_<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }

    fn visit_binop(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Binop<(), ()>,
    ) -> Result<(), ()> {
        println!{"Binop"};

        let _ = p.recurse(c, self.object());
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
        c: &mut Context,
        p: &'ast aast::Block<(), ()>,
    ) -> Result<(), ()> {
        println!("Block");
        let _  = p.recurse(c, self.object());
        let stmts = get_vec_len(&mut self.tree_stack, "Stmt", p.0.len());
        let b = json!({
            "kind": "Block",
            "stmts": stmts,
        });
        self.tree_stack.push(b);
        Ok(())
    }
    //Done
    fn visit_bop(
        &mut self,
        c: &mut Context,
        p: &'ast ast_defs::Bop,
    ) -> Result<(), ()> {
        println!("Bop");
        let _ = p.recurse(c, self.object());
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
        c: &mut Context,
        p: &'ast aast::CallExpr<(), ()>,
    ) -> Result<(), ()> {
        println!("Call Expr");
        let _ = p.func.recurse(c, self.object());
        let func = self.tree_stack.pop();
        let _ = p.targs.recurse(c, self.object());
        let mut unpacked_arg = Value::Null;
        if p.unpacked_arg.is_none(){
            unpacked_arg = self.tree_stack.pop().unwrap();
        }

        let targs = get_vec_len(&mut self.tree_stack,"Targ", p.targs.len());

        // let args = get_vec(&mut self.tree_stack,"Expr");
        
        if p.unpacked_arg.is_none(){
            let ce = json!({
                "kind": "CallExpr",
                "func": func,
                // "args": args,
                "targs": targs,
            });
            self.tree_stack.push(ce);
        }
        else{
            let ce = json!({
                "kind": "CallExpr",
                "func": self.tree_stack.pop(),
                "targs": targs,
                // "args": args,
                "unpacked_args": unpacked_arg,
            });
            self.tree_stack.push(ce);
        }
        Ok(())
    }
    fn visit_capture_lid(
        &mut self,
        c: &mut Context,
        p: &'ast aast::CaptureLid<()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_case(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Case<(), ()>,
    ) -> Result<(), ()> {
        println!("Case");
        let _ = p.recurse(c, self.object());
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
        c: &mut Context,
        p: &'ast aast::Catch<(), ()>,
    ) -> Result<(), ()> {
        println!("Catch");
        let _ = p.recurse(c, self.object());
        let c = json!({
            "kind": "Catch",
            "class": p.0.1.clone(),
            "id": p.1.1.clone(),
            "block": self.tree_stack.pop(), 
        });
        self.tree_stack.push(c);
        Ok(())
    }
    fn visit_class_abstract_typeconst(
        &mut self,
        c: &mut Context,
        p: &'ast aast::ClassAbstractTypeconst,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_class_concrete_typeconst(
        &mut self,
        c: &mut Context,
        p: &'ast aast::ClassConcreteTypeconst,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_class_const(
        &mut self,
        c: &mut Context,
        p: &'ast aast::ClassConst<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_class_const_kind(
        &mut self,
        c: &mut Context,
        p: &'ast aast::ClassConstKind<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_class_get_expr(
        &mut self,
        c: &mut Context,
        p: &'ast aast::ClassGetExpr<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_class_id(
        &mut self,
        c: &mut Context,
        p: &'ast aast::ClassId<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_class_id_(
        &mut self,
        c: &mut Context,
        p: &'ast aast::ClassId_<(), ()>,
    ) -> Result<(), ()> {
        let _ = p.recurse(c, self.object());
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
    fn visit_class_req(
        &mut self,
        c: &mut Context,
        p: &'ast aast::ClassReq,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_class_typeconst(
        &mut self,
        c: &mut Context,
        p: &'ast aast::ClassTypeconst,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_class_typeconst_def(
        &mut self,
        c: &mut Context,
        p: &'ast aast::ClassTypeconstDef<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }

    fn visit_class_var(
        &mut self,
        c: &mut Context,
        p: &'ast aast::ClassVar<(), ()>,
    ) -> Result<(), ()> {
        println!("Class Var");
        let _ = p.recurse(c, self.object());

        let cv = json!({
            "kind": "ClassVar",
            "abstract": p.abstract_.clone(),
            "readonly": p.readonly.clone(),
            "visibility": self.string_stack.pop(),
            "type": self.tree_stack.pop(),
            "id": p.id.1.clone(),
            "is_static": p.is_static.clone()
        });
        self.tree_stack.push(cv);
        Ok(())
    }

    fn visit_class_(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Class_<(), ()>,
    ) -> Result<(), ()> {
        println!("Class");
        let _ = p.recurse(c, self.object());
  
        let methods = get_vec_len(&mut self.tree_stack, "Method", p.tparams.len());
        let vars = get_vec_len(&mut self.tree_stack, "ClassVar", p.vars.len());

        let class_kind = self.tree_stack.pop();
        let class = json!({
            "kind": "Class",
            "name": p.name.1,
            "class_kind": class_kind,
            "vars": vars,
            "methods": methods,
        });
        self.tree_stack.push(class);
        Ok(())
    }
    //Implement in Class so it's prettier 
    fn visit_classish_kind(
        &mut self,
        c: &mut Context,
        p: &'ast ast_defs::ClassishKind,
    ) -> Result<(), ()> {
        println!("Classish Kind");
        let _ = p.recurse(c, self.object());
        let mut kind = String::new();
        let mut abstraction = String::new();
        match p {
            ast_defs::ClassishKind::Cclass(a) =>{
                kind = "Class".to_string();
                match a{
                    ast_defs::Abstraction::Concrete =>{
                        abstraction = "Concrete".to_string();
                    }
                    ast_defs::Abstraction::Abstract =>{
                        abstraction = "Abstract".to_string();
                    }
                }
            }
            ast_defs::ClassishKind::Cinterface =>{
                kind = "Interface".to_string()
            }
            ast_defs::ClassishKind::Ctrait =>{
                kind = "Trait".to_string();
            }
            ast_defs::ClassishKind::Cenum => {
                kind = "Enum".to_string();
            }
            ast_defs::ClassishKind::CenumClass(a) =>{
                match a{
                    ast_defs::Abstraction::Concrete =>{
                        abstraction = "Concrete".to_string();
                    }
                    ast_defs::Abstraction::Abstract =>{
                        abstraction = "Abstract".to_string();
                    }
                }
            }

        }

        if abstraction.is_empty(){
            let ck = json!({
                "kind": kind,
            });
            self.tree_stack.push(json!(ck));
        }
        else{
            let ck = json!({
                "kind": kind,
                "abstraction": abstraction,
            });
            self.tree_stack.push(json!(ck));
        }
        Ok(())
    }

    fn visit_collection_targ(
        &mut self,
        c: &mut Context,
        p: &'ast aast::CollectionTarg<()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_constraint_kind(
        &mut self,
        c: &mut Context,
        p: &'ast ast_defs::ConstraintKind,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_contexts(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Contexts,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_ctx_refinement(
        &mut self,
        c: &mut Context,
        p: &'ast aast::CtxRefinement,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_ctx_refinement_bounds(
        &mut self,
        c: &mut Context,
        p: &'ast aast::CtxRefinementBounds,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }

    // No need to implement
    fn visit_def(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Def<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_default_case(
        &mut self,
        c: &mut Context,
        p: &'ast aast::DefaultCase<(), ()>,
    ) -> Result<(), ()> {
        println!("Default Case");
        let _ = p.recurse(c, self.object());
        let dc = json!({
            "kind": "DefaultCase",
            "block": self.tree_stack.pop(),
        });
        self.tree_stack.push(dc);
        Ok(())
    }
    fn visit_efun(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Efun<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_emit_id(
        &mut self,
        c: &mut Context,
        p: &'ast aast::EmitId,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_enum_(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Enum_,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    //Ignore
    fn visit_expr(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Expr<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    //Ignore
    fn visit_expr_(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Expr_<(), ()>,
    ) -> Result<(), ()> {
        let _ = p.recurse(c, self.object());
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
            aast_defs::Expr_::ObjGet(_a)=>{
                let expr2 = self.tree_stack.pop();
                let expr1 = self.tree_stack.pop();
                let expr = json!({
                    "kind": "Expr",
                    "type": "ObjGet",
                    "expr": [expr1, expr2],
                });
                self.tree_stack.push(expr); 
            }
            aast_defs::Expr_::Call(_a)=>{
                let expr = json!({
                    "kind": "Expr",
                    "type": "Call",
                    "expr": self.tree_stack.pop(),
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
                    "expr": a,
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
            aast_defs::Expr_::New(a) =>{
                if a.3.is_some(){
                    let opt_expr = self.tree_stack.pop();
                    let vec_exprs = get_vec_len(&mut self.tree_stack, "Expr", a.2.len());
                    let vec_targs = get_vec_len(&mut self.tree_stack, "Targ", a.1.len());
                    let expr = json!({
                        "kind": "Expr",
                        "classId": self.tree_stack.pop(),
                        "targs": vec_targs,
                        "exprs": vec_exprs,
                        "opt": opt_expr,
                    });
                    self.tree_stack.push(expr); 
                }
                else{
                    let vec_exprs = get_vec_len(&mut self.tree_stack, "Expr", a.2.len());
                    let vec_targs = get_vec_len(&mut self.tree_stack, "Targ", a.1.len());
                    let expr = json!({
                        "kind": "Expr",
                        "classId": self.tree_stack.pop(),
                        "targs": vec_targs,
                        "exprs": vec_exprs,
                    });
                    self.tree_stack.push(expr); 
                }
            }
            _ =>{    //Call Expr
            }
        }
        Ok(())
    }
    fn visit_expression_tree(
        &mut self,
        c: &mut Context,
        p: &'ast aast::ExpressionTree<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_field(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Field<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_file_attribute(
        &mut self,
        c: &mut Context,
        p: &'ast aast::FileAttribute<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_finally_block(
        &mut self,
        c: &mut Context,
        p: &'ast aast::FinallyBlock<(), ()>,
    ) -> Result<(), ()> {
        println!("Finally Block");
        let _ = p.recurse(c, self.object());
        let stmts = get_vec_len(&mut self.tree_stack, "Stmt", p.0.len());
        let fb = json!({
            "kind": "FinallyBlock",
            "stmts": stmts,
        });
        self.tree_stack.push(fb);
        Ok(())
    }
    //DONE - did not implement namespace, file_attributes, mode, internal, module, tparams, where_constraints
    fn visit_fun_def(
        &mut self,
        c: &mut Context,
        p: &'ast aast::FunDef<(), ()>,
    ) -> Result<(), ()> {
        println!("FunDef");
        let _ = p.recurse(c, self.object());
        let fd = json!({
            "kind": "FunDef",
            "name": p.name.1.clone(),
            "doc_comment": "Testing 123",
            "child": self.tree_stack.pop(),
		});
        self.tree_stack.push(fd);
        Ok(())
    }
    fn visit_fun_kind(
        &mut self,
        c: &mut Context,
        p: &'ast ast_defs::FunKind,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    // Done
    fn visit_fun_param(
        &mut self,
        c: &mut Context,
        p: &'ast aast::FunParam<(), ()>,
    ) -> Result<(), ()> {
        println!("FunParam");
        let _ = p.recurse(c, self.object());

		let fp = json!({
            "kind": "FuncParam",
            "name": p.name.clone(),
            "type": "Type Test",
            "visibility": self.string_stack.pop(),
        });
		self.tree_stack.push(fp);
		Ok(())
    }
    // DONE - ignore span, readonly_this, annotation, readonly_ret, ctxs, unsafe_ctxs, fun_kind, user_attributes, external, doc_comment
    fn visit_fun_(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Fun_<(), ()>,
    ) -> Result<(), ()> {
        println!("Fun_");
        let _ = p.recurse(c, self.object());

        let body = self.tree_stack.pop();
        let params = get_vec_len(&mut self.tree_stack,"FuncParam", p.params.len());
        // let ret = self.tree_stack.pop();
        let ret = "Type Test";
        let f = json!({
                "kind": "Fun",
                "ret": ret,
                "params": params,
                "body": body,
            });
        self.tree_stack.push(f);
        Ok(())
    }
    //DONE - sortof
    fn visit_func_body(
        &mut self,
        c: &mut Context,
        p: &'ast aast::FuncBody<(), ()>,
    ) -> Result<(), ()> {
        println!("FuncBody");
        let _ = p.recurse(c, self.object());

        let fb = json!({
            "kind": "FuncBody",
            "body": self.tree_stack.pop(),
        });
        self.tree_stack.push(fb);

        Ok(())
    }
    fn visit_function_ptr_id(
        &mut self,
        c: &mut Context,
        p: &'ast aast::FunctionPtrId<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_gconst(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Gconst<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_hf_param_info(
        &mut self,
        c: &mut Context,
        p: &'ast aast::HfParamInfo,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    // fn visit_hint(
    //     &mut self,
    //     c: &mut Context,
    //     p: &'ast aast::Hint,
    // ) -> Result<(), ()> {
    //     p.recurse(c, self.object())
    // }
    // fn visit_hint_fun(
    //     &mut self,
    //     c: &mut Context,
    //     p: &'ast aast::HintFun,
    // ) -> Result<(), ()> {
    //     p.recurse(c, self.object())
    // }
    // fn visit_hint_(
    //     &mut self,
    //     c: &mut Context,
    //     p: &'ast aast::Hint_,
    // ) -> Result<(), ()> {
    //     println!("Hint");
    //     let _ = p.recurse(c, self.object());
    //     match p {
    //         aast::Hint_::Hmixed =>{
    //             let ht = json!({
    //                 "kind": "Hint",
    //                 "type": "Mixed",
    //             });
    //             self.tree_stack.push(ht);
    //         },
    //         aast::Hint_::Happly(n, v) =>{
    //             if v.is_empty(){
    //                 let ht = json!({
    //                     "kind": "Hint",
    //                     "type": "Apply",
    //                     "name": n.1.clone(),
    //                 });
    //                 self.tree_stack.push(ht);
    //             }
    //             else{
    //                 let ht = json!({
    //                     "kind": "Hint",
    //                     "type": "Apply",
    //                     "name": n.1.clone(),
    //                     "children": self.tree_stack.pop(),
    //                 });
    //                 self.tree_stack.push(ht);
    //             }
    //         },

    //         _ =>{},
    //     }

    //     Ok(())
    // }
    fn visit_hole_source(
        &mut self,
        c: &mut Context,
        p: &'ast aast::HoleSource,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    // Ignore
    fn visit_id(
        &mut self,
        c: &mut Context,
        p: &'ast ast_defs::Id,
    ) -> Result<(), ()> {
        // println!("Id");
        // let _ = p.recurse(c, self.object());
        // let id = json!(p.1);
        // self.tree_stack.push(id);
        // Ok(())
        p.recurse(c, self.object())
    }
    fn visit_import_flavor(
        &mut self,
        c: &mut Context,
        p: &'ast aast::ImportFlavor,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_kvc_kind(
        &mut self,
        c: &mut Context,
        p: &'ast aast::KvcKind,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    //IGNORE
    fn visit_lid(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Lid,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_md_name_kind(
        &mut self,
        c: &mut Context,
        p: &'ast aast::MdNameKind,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_method_(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Method_<(), ()>,
    ) -> Result<(), ()> {
        println!("Method");
        let _ = p.recurse(c, self.object());

        let ret = self.tree_stack.pop();
        let body = self.tree_stack.pop();
        let params = get_vec_len(&mut self.tree_stack,"FuncParam", p.params.len());
        let m = json!({
            "kind": "Method",
            "name": p.name.1.clone(),
            "params": params,
            "visibility": self.string_stack.pop(),
            "is_final": p.final_.clone(),
            "is_abstract": p.abstract_.clone(),
            "is_static": p.static_.clone(),
            "body": body,
            "ret": ret,
            
        });
        self.tree_stack.push(m);
        Ok(())
    }
    fn visit_module_def(
        &mut self,
        c: &mut Context,
        p: &'ast aast::ModuleDef<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_nast_shape_info(
        &mut self,
        c: &mut Context,
        p: &'ast aast::NastShapeInfo,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_ns_kind(
        &mut self,
        c: &mut Context,
        p: &'ast aast::NsKind,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_og_null_flavor(
        &mut self,
        c: &mut Context,
        p: &'ast aast::OgNullFlavor,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_optional_kind(
        &mut self,
        c: &mut Context,
        p: &'ast ast_defs::OptionalKind,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_param_kind(
        &mut self,
        c: &mut Context,
        p: &'ast ast_defs::ParamKind,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_pat_refinement(
        &mut self,
        c: &mut Context,
        p: &'ast aast::PatRefinement,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_pat_var(
        &mut self,
        c: &mut Context,
        p: &'ast aast::PatVar,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_pattern(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Pattern,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    // DONE
    fn visit_program(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Program<(), ()>,
    ) -> Result<(), ()> {
        println!("Program");
        let _ = p.recurse(c, self.object());
        let mut defs = VecDeque::<Value>::new();
        while let Some(last) = self.tree_stack.pop() {
            defs.push_front(last);
        }
        let obj = json!({
            "kind": "Program",
            "definitions": defs,
        });

        self.tree = serde_json::to_string_pretty(&obj).unwrap();
        println!("{}", serde_json::to_string_pretty(&obj).unwrap());
        Ok(())
    }

    fn visit_prop_or_method(
        &mut self,
        c: &mut Context,
        p: &'ast aast::PropOrMethod,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_readonly_kind(
        &mut self,
        c: &mut Context,
        p: &'ast ast_defs::ReadonlyKind,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_refinement(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Refinement,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_reify_kind(
        &mut self,
        c: &mut Context,
        p: &'ast aast::ReifyKind,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_require_kind(
        &mut self,
        c: &mut Context,
        p: &'ast aast::RequireKind,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_shape_field_info(
        &mut self,
        c: &mut Context,
        p: &'ast aast::ShapeFieldInfo,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_shape_field_name(
        &mut self,
        c: &mut Context,
        p: &'ast ast_defs::ShapeFieldName,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_stmt(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Stmt<(), ()>,
    ) -> Result<(), ()> {
        // let _ = p.recurse(c, self.object());
        // let stmt = json!({
        //     "kind": "Stmt",
        //     "type": self.tree_stack.pop(),
        // });
        // self.tree_stack.push(stmt);
        // Ok(())
        p.recurse(c, self.object())
    }
    fn visit_stmt_match(
        &mut self,
        c: &mut Context,
        p: &'ast aast::StmtMatch<(), ()>,
    ) -> Result<(), ()> {
        println!("Stmt Match");
        let _ = p.recurse(c, self.object());
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
    //Ignore Pattern
    fn visit_stmt_match_arm(
        &mut self,
        c: &mut Context,
        p: &'ast aast::StmtMatchArm<(), ()>,
    ) -> Result<(), ()> {
        println!("Stmt Match Arm");
        let _ = p.recurse(c, self.object());
        let stmt = json!({
            "kind": "StmtMatchArm",
            "body": self.tree_stack.pop(),
        });
        self.tree_stack.push(stmt);
        Ok(())
    }
    fn visit_stmt_(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Stmt_<(), ()>,
    ) -> Result<(), ()> {
        println!("Stmt");
        let _ = p.recurse(c, self.object());
        let mut stmt = Value::Null;
        match p{
            aast::Stmt_::Noop => {
                stmt = json!({
                    "kind": "Stmt",
                    "type": "Noop",
                });
            }
            aast::Stmt_::Fallthrough => {
                stmt = json!({
                    "kind": "Stmt",
                    "type": "Fallthrough",
                });
            }
            aast::Stmt_::Expr(_) => {
                stmt = json!({
                    "kind": "Stmt",
                    "type": "Expr",
                    "stmt": self.tree_stack.pop(),
                });
            }
            aast::Stmt_::Break => {
                stmt = json!({
                    "kind": "Stmt",
                    "type": "Break",
                });
            }
            aast::Stmt_::Continue => {
                stmt = json!({
                    "kind": "Stmt",
                    "type": "Continue",
                });
            }
            aast::Stmt_::Throw(_) => {
                stmt = json!({
                    "kind": "Stmt",
                    "type": "Throw",
                    "stmt": self.tree_stack.pop(),
                });
            }
            aast::Stmt_::Return(expr) => {
                if (*expr).is_none(){
                    stmt = json!({
                        "kind": "Stmt",
                        "type": "Return",
                    });
                }
                else {
                    stmt = json!({
                        "kind": "Stmt",
                        "type": "Return",
                        "stmt": self.tree_stack.pop()
                    });
                }
            }
            aast::Stmt_::YieldBreak => {
                stmt = json!({
                    "kind": "Stmt",
                    "type": "YieldBreak",
                });
            }
            //Be careful with awaitall
            aast::Stmt_::Awaitall(a) => {
                let block = self.tree_stack.pop();
                let exprs = get_vec_len(&mut self.tree_stack, "Expr", a.0.len());
                stmt = json!({
                    "kind": "Stmt",
                    "type": "Awaitall",
                    "stmt": [block, exprs],
                });
            }
            aast::Stmt_::Concurrent(_) => {
                stmt = json!({
                    "kind": "Stmt",
                    "type": "Concurrent",
                    "stmt": self.tree_stack.pop(),
                });
            }
            aast::Stmt_::If(_) => {
                let b2 = self.tree_stack.pop();
                let b1 = self.tree_stack.pop();
                let expr = self.tree_stack.pop();
                stmt = json!({
                    "kind": "Stmt",
                    "type": "If",
                    "stmt": [expr, b1, b2],
                })
                
            }
            aast::Stmt_::Do(_) => {
                let expr = self.tree_stack.pop();
                let b = self.tree_stack.pop();
                stmt = json!({
                    "kind": "Stmt",
                    "type": "Do",
                    "stmt": [b, expr],
                })
            }
            aast::Stmt_::While(_) => {
                let b = self.tree_stack.pop();
                let expr = self.tree_stack.pop();
                stmt = json!({
                    "kind": "Stmt",
                    "type": "While",
                    "stmt": [expr, b],
                })
            }
            aast::Stmt_::Using(_) => {
                stmt = json!({
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
                    stmt = json!({
                        "kind": "Stmt",
                        "type": "For",
                        "stmt": [exprs1, opt_expr, exprs2, b],
                    })
                }
                else{
                    let exprs1 = get_vec_len(&mut self.tree_stack, "Expr", f.0.len());
                    stmt = json!({
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
                    stmt = json!({
                        "kind": "Stmt",
                        "type": "Switch",
                        "stmt": [expr, cases, dc],
                    });
                }
                else{
                    stmt = json!({
                        "kind": "Stmt",
                        "type": "Switch",
                        "stmt": [expr, cases],
                    });
                }

            }
            aast::Stmt_::Match(match_stmt) => {
                stmt = json!({
                    "kind": "Stmt",
                    "type": "Match",
                    "stmt": self.tree_stack.pop(),
                });
            }
            aast::Stmt_::Foreach(_) => {
                let block = self.tree_stack.pop();
                let as_expr = self.tree_stack.pop();
                let expr = self.tree_stack.pop();
                stmt = json!({
                    "kind": "Stmt",
                    "type": "Try",
                    "stmt": [block, as_expr, expr],
                });
            }
            aast::Stmt_::Try(ts) => {
                let finally_block = self.tree_stack.pop();
                let catchs = get_vec_len(&mut self.tree_stack, "Catch", ts.1.len());
                let block = self.tree_stack.pop();
                stmt = json!({
                    "kind": "Stmt",
                    "type": "Try",
                    "stmt": [block, catchs, finally_block],
                });
            }
            aast::Stmt_::DeclareLocal(dl) => {
                if dl.2.is_some(){
                    stmt = json!({
                        "kind": "Stmt",
                        "type": "DeclareLocal",
                        "stmt": self.tree_stack.pop()
                    })
                }
                else{
                    stmt = json!({
                        "kind": "Stmt",
                        "type": "DeclareLocal",
                    })
                }
            }
            //Ignore Lid
            aast::Stmt_::Block(_) => {
                stmt = json!({
                    "kind": "Stmt",
                    "type": "Block",
                    "stmt": self.tree_stack.pop(),
                })
            }
            aast::Stmt_::Markup(m) => {
                stmt = json!({
                    "kind": "Stmt",
                    "type": "Markup",
                    "stmt": m.1,
                });
            }
        }
        self.tree_stack.push(stmt);
        Ok(())
    }
    fn visit_targ(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Targ<()>,
    ) -> Result<(), ()> {
        println!("Targ");
        let _ = p.recurse(c, self.object());
        let targ = json!({
            "kind": "Targ",
            "type": self.tree_stack.pop(),
        });
        self.tree_stack.push(targ);
        Ok(())
    }
    fn visit_tparam(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Tparam<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    // fn visit_tprim(
    //     &mut self,
    //     c: &mut Context,
    //     p: &'ast aast::Tprim,
    // ) -> Result<(), ()> {
    //     let _ = p.recurse(c, self.object());
    //     let prim = match p {
    //         ast_defs::Tprim::Tnull => "Tnull",
    //         ast_defs::Tprim::Tvoid => "Tvoid",
    //         ast_defs::Tprim::Tint => "Tint",
    //         ast_defs::Tprim::Tbool => "Tbool",
    //         ast_defs::Tprim::Tfloat => "Tfloat",
    //         ast_defs::Tprim::Tstring => "Tstring",
    //         ast_defs::Tprim::Tresource => "Tresource",
    //         ast_defs::Tprim::Tnum => "Tnum",
    //         ast_defs::Tprim::Tarraykey => "Tarraykey",
    //         ast_defs::Tprim::Tnoreturn => "Tnoreturn",
    //     };
    //     let ht = json!({
    //         "kind": "Hint",
    //         "type": "Primative",
    //         "name": prim.to_string(),
    //     });
    //     self.tree_stack.push(ht);
    //     Ok(())
    // }
    // fn visit_type_hint(
    //     &mut self,
    //     c: &mut Context,
    //     p: &'ast aast::TypeHint<()>,
    // ) -> Result<(), ()> {
    //     println!{"Type Hint"};
    //     let _ = p.recurse(c, self.object());

    //     let th = json!({
	// 		"kind": "TypeHint",
	// 		"child": self.tree_stack.pop(),
	// 	});
	// 	self.tree_stack.push(th);
    //     Ok(())
    // }
    fn visit_type_refinement(
        &mut self,
        c: &mut Context,
        p: &'ast aast::TypeRefinement,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    fn visit_type_refinement_bounds(
        &mut self,
        c: &mut Context,
        p: &'ast aast::TypeRefinementBounds,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    //Do
    fn visit_typedef(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Typedef<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    //Do maybe
    fn visit_typedef_visibility(
        &mut self,
        c: &mut Context,
        p: &'ast aast::TypedefVisibility,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    //Do
    fn visit_uop(
        &mut self,
        c: &mut Context,
        p: &'ast ast_defs::Uop,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    //Skip
    fn visit_user_attribute(
        &mut self,
        c: &mut Context,
        p: &'ast aast::UserAttribute<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    //Skip
    fn visit_user_attributes(
        &mut self,
        c: &mut Context,
        p: &'ast aast::UserAttributes<(), ()>,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    //Do
    fn visit_using_stmt(
        &mut self,
        c: &mut Context,
        p: &'ast aast::UsingStmt<(), ()>,
    ) -> Result<(), ()> {
        println!("Using Stmt");
        let _ = p.recurse(c, self.object());
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
        c: &mut Context,
        p: &'ast ast_defs::Variance,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    //Skip
    fn visit_vc_kind(
        &mut self,
        c: &mut Context,
        p: &'ast aast::VcKind,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    //Implemented in Method and Class Var
    fn visit_visibility(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Visibility,
    ) -> Result<(), ()> {
        println!("Visibility");
        let _ = p.recurse(c, self.object());
        let visibility = match p {
            ast_defs::Visibility::Private => "private",
            ast_defs::Visibility::Public => "public",
            ast_defs::Visibility::Protected => "protected",
            ast_defs::Visibility::Internal => "internal",
        };
        self.string_stack.push(visibility.to_string());
        Ok(())
    }
    //Do
    fn visit_where_constraint_hint(
        &mut self,
        c: &mut Context,
        p: &'ast aast::WhereConstraintHint,
    ) -> Result<(), ()> {
        p.recurse(c, self.object())
    }
    
    fn visit_ex(
        &mut self,
        c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        p: &'ast <Self::Params as oxidized::aast_visitor::Params>::Ex,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        Ok(())
    }
    
    fn visit_en(
        &mut self,
        c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        p: &'ast <Self::Params as oxidized::aast_visitor::Params>::En,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        Ok(())
    }
    
    fn visit_xhp_attr(
        &mut self,
        c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        p: &'ast aast::XhpAttr<<Self::Params as oxidized::aast_visitor::Params>::Ex, <Self::Params as oxidized::aast_visitor::Params>::En>,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        p.recurse(c, self.object())
    }
    
    fn visit_xhp_attr_info(
        &mut self,
        c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        p: &'ast aast::XhpAttrInfo,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        p.recurse(c, self.object())
    }
    
    fn visit_xhp_attr_tag(
        &mut self,
        c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        p: &'ast aast::XhpAttrTag,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        p.recurse(c, self.object())
    }
    
    fn visit_xhp_attribute(
        &mut self,
        c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        p: &'ast aast::XhpAttribute<<Self::Params as oxidized::aast_visitor::Params>::Ex, <Self::Params as oxidized::aast_visitor::Params>::En>,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        p.recurse(c, self.object())
    }
    
    fn visit_xhp_child(
        &mut self,
        c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        p: &'ast aast::XhpChild,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        p.recurse(c, self.object())
    }
    
    fn visit_xhp_child_op(
        &mut self,
        c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        p: &'ast aast::XhpChildOp,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        p.recurse(c, self.object())
    }
    
    fn visit_xhp_enum_value(
        &mut self,
        c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        p: &'ast ast_defs::XhpEnumValue,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        p.recurse(c, self.object())
    }
    
    fn visit_xhp_simple(
        &mut self,
        c: &mut <Self::Params as oxidized::aast_visitor::Params>::Context,
        p: &'ast aast::XhpSimple<<Self::Params as oxidized::aast_visitor::Params>::Ex, <Self::Params as oxidized::aast_visitor::Params>::En>,
    ) -> Result<(), <Self::Params as oxidized::aast_visitor::Params>::Error> {
        p.recurse(c, self.object())
    }
}
    //Skip
    // fn visit_xhp_attr(
    //     &mut self,
    //     c: &mut Context,
    //     p: &'ast aast::XhpAttr<(), ()>,
    // ) -> Result<(), ()> {
    //     p.recurse(c, self.object())
    // }
    // fn visit_xhp_attr_info(
    //     &mut self,
    //     c: &mut Context,
    //     p: &'ast aast::XhpAttrInfo,
    // ) -> Result<(), ()> {
    //     p.recurse(c, self.object())
    // }
    // fn visit_xhp_attr_tag(
    //     &mut self,
    //     c: &mut Context,
    //     p: &'ast aast::XhpAttrTag,
    // ) -> Result<(), ()> {
    //     p.recurse(c, self.object())
    // }
    // fn visit_xhp_attribute(
    //     &mut self,
    //     c: &mut Context,
    //     p: &'ast aast::XhpAttribute<(), ()>,
    // ) -> Result<(), ()> {
    //     p.recurse(c, self.object())
    // }
    // fn visit_xhp_child(
    //     &mut self,
    //     c: &mut Context,
    //     p: &'ast aast::XhpChild,
    // ) -> Result<(), ()> {
    //     p.recurse(c, self.object())
    // }
    // fn visit_xhp_child_op(
    //     &mut self,
    //     c: &mut Context,
    //     p: &'ast aast::XhpChildOp,
    // ) -> Result<(), ()> {
    //     p.recurse(c, self.object())
    // }
    // fn visit_xhp_enum_value(
    //     &mut self,
    //     c: &mut Context,
    //     p: &'ast aast::XhpEnumValue,
    // ) -> Result<(), ()> {
    //     p.recurse(c, self.object())
    // }
    // fn visit_xhp_simple(
    //     &mut self,
    //     c: &mut Context,
    //     p: &'ast aast::XhpSimple<(), ()>,
    // ) -> Result<(), ()> {
    //     p.recurse(c, self.object())
    // }




// impl <'ast>Visitor<'ast> for Scanner {
// 	type Params = AstParams<Context, ()>;
// 	fn object(&mut self) -> &mut dyn Visitor<'ast, Params = Self::Params> {
//         self
// 	}
//     fn visit_program(
//         &mut self,
//         nc: &mut Context,
//         p: &'ast aast::Program<(), ()>,
//     ) -> Result<(), ()> {
//         let res = match p.recurse(nc, self){
//             Ok(res) => res,
//             Err(err) => {
//                 return Err(err);
//             }
//         };

//         let obj = json!({
//             "kind": "Program",
//             "child": self.tree_stack.pop(),
//         });

//         self.tree = serde_json::to_string_pretty(&obj).unwrap();
//         println!("{}", serde_json::to_string_pretty(&obj).unwrap());
//         Ok(Default::default())

//     }
	// fn visit_fun_def(
    //     &mut self,
    //     c: &mut Context,
    //     p: &'ast aast::FunDef<(), ()>,
    // ) -> Result<(), ()> {
    //     println!("FunDef");
	// 	let _res = match p.recurse(c, self){
	// 		Ok(_res) => _res,
	// 		Err(err) => {
	// 			return Err(err);
	// 		}
	// 	};
    //     println!("Ret FunDef");
	// 	let fd = json!({
	// 			"kind": "FunDef",
	// 			"name": p.name.1,
    //             "doc_comment": "Testing 123",
    //             "child": self.tree_stack.pop(),
	// 	});
	// 	self.tree_stack.push(fd);
	// 	Ok(Default::default())
    // }

// 	fn visit_fun_(
//         &mut self,
//         c: &mut Context,
//         p: &'ast aast::Fun_<(), ()>,
//     ) -> Result<(), ()> {
//         println!("Fun_");
// 		let _res = match p.recurse(c, self){
// 			Ok(_res) => _res,
// 			Err(err) => {
// 				return Err(err);
// 			}
// 		};
//         println!("Ret Fun");
        
//         let body = self.tree_stack.pop();
//         let mut params = VecDeque::<Value>::new();
//         // let mut param = self.tree_stack.pop().unwrap();
//         while let Some(last) = self.tree_stack.last() {
//             if last["kind"] != "FuncParam" {
//                 break;
//             }
//             params.push_front(self.tree_stack.pop().unwrap());
//         }
// 		let f = json!({
// 				"kind": "Fun",
//                 "body": body,
//                 "params": params,
//                 "ret": self.tree_stack.pop(),
// 			});
//         self.tree_stack.push(f);
// 		Ok(Default::default())
//     }

// 	fn visit_func_body(
//         &mut self,
//         c: &mut Context,
//         p: &'ast aast::FuncBody<(), ()>,
//     ) -> Result<(), ()> {
//         println!("Fun Body");
// 		let _res = match p.recurse(c, self){
// 			Ok(_res) => _res,
// 			Err(err) => {
// 				return Err(err);
// 			}
// 		};
//         println!("Ret Fun Body");
// 		let fb = json!({
// 				"kind": "FuncBody",
// 				"children": self.tree_stack.pop(),
// 			});
//         self.tree_stack.push(fb);
// 		Ok(Default::default())
//     }

// 	fn visit_fun_param(
//         &mut self,
//         c: &mut Context,
//         p: &'ast aast::FunParam<(), ()>,
//     ) -> Result<(), ()> {
//         println!("Fun Param");
// 		let _res = match p.recurse(c, self){
// 			Ok(_res) => _res,
// 			Err(err) => {
// 				return Err(err);
// 			}
// 		};
//         println!("Ret Fun Param");
// 		let fp = json!({
// 				"kind": "FuncParam",
//                 "name": p.name.clone(),
// 				"children": self.tree_stack.pop(),
// 			});
// 		self.tree_stack.push(fp);
// 		// println!("{:#?}", fp);
// 		Ok(Default::default())
//     }

// 	fn visit_type_hint(
//         &mut self,
//         c: &mut Context,
//         p: &'ast aast::TypeHint<()>,
//     ) -> Result<(), ()> {
//         println!("Type Hint");
// 		let th = json!({
// 			"kind": "TypeHint",
// 			"child": *(p.1.clone().unwrap().1),
// 		});
//         println!("Ret Type Hint");
// 		self.tree_stack.push(th);

// 		Ok(Default::default())
//     }

//     fn visit_expr_(
//         &mut self,
//         c: &mut Context,
//         e: &'ast aast::Expr_<(), ()>,
//     ) -> Result<(), ()> {
//         let _res = match e.recurse(c, self){
// 			Ok(_res) => _res,
// 			Err(err) => {
// 				return Err(err);
// 			}
// 		};
//         println!("Expr");
//         let ex = json!({
//             "kind": "Expr",
//             "child": self.tree_stack.pop(),
//         });
//         println!("Ret Expr");
//         self.tree_stack.push(ex);
//         Ok(Default::default())
//     }
    

//     fn visit_binop(
//         &mut self,
//         c: &mut Context,
//         b: &'ast aast::Binop<(), ()>,
//     ) -> Result<(), ()> {
//         let _res = match b.recurse(c, self){
// 			Ok(_res) => _res,
// 			Err(err) => {
// 				return Err(err);
// 			}
// 		};
//         println!("Binop");
//         let rhs = self.tree_stack.pop();
//         let lhs = self.tree_stack.pop();
//         let bi = json!({
// 			"kind": "Binop",
//             "bop": b.bop.clone(),
// 			// "lhs": lhs,
//             // "rhs": rhs,
// 		});
//         println!("Ret Binop");
//         self.tree_stack.push(bi);
//         Ok(Default::default())
//     }
// }


// aast::Def::Fun(f) => {
// 	let fr = &f.fun.ret;
// 	let rh_json = json!( { 
// 		"kind": "Id", //What can we do about Ex
// 		"value": *(fr.1.clone().unwrap().1)
// 	});
// 	let fr_json = json!({
// 		"kind": "ret".to_string(),
// 		"type_hint": rh_json,
// 	});

// 	let fb = &f.fun.body;
// 	let fb_json = json!({
// 		"kind": "FuncBody",
// 		"child": fb.fb_ast.0.clone(),
// 	});

// 	let fp = &f.fun.params;
// 	let mut fp_arr: Vec<Value> = Vec::new();
// 	for param in fp.iter(){
// 		let th_json = json!({
// 			"kind": "TypeHint",
// 			"value": *(param.type_hint.1.clone().unwrap().1)
// 		});
// 		let fp_json = json!({
// 			"kind": "FunParam",
// 			"name": param.name.clone(),
// 			"type_hint": th_json,
// 			"is_variadic": param.is_variadic
// 		});
// 		fp_arr.push(fp_json);
// 	};
	
// 	let obj = json!({
// 		"kind": "program",
// 		"children": {
// 			"kind": "Fun",
// 			"doc_comment": "Silly",
// 			"child": {
// 				"kind": "FunDef",
// 				"name": f.name.clone(), 
// 				"span": f.fun.span.clone(),
// 				"params": fp_arr,
// 				"body": fb_json,
// 				"ret": fr_json,
// 			}
// 		}
// 	});
// }