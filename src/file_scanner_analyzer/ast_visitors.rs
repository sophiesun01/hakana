extern crate json_value_merge;

use json_value_merge::Merge;

use oxidized::{
    aast,
    aast_visitor::{visit, AstParams, Node, Visitor},
    ast_defs,
};
use rustc_hash::FxHashMap;
use serde_json::{self, json};
use serde_json::Value;

pub(crate) struct Scanner {
    pub tree_stack: Vec<serde_json::Value>,
    pub tree: String
    // pub interner: &'a mut ThreadedInterner
}
pub(crate) struct Context {

}

//Implementing a similar merge function in order to not use the serde_json crate
// fn merge_children(a: &mut Value, b: Value) {
//     println!("{:#?}", a["children"]);
//     println!("{:#?}", b["children"]);
//     // a["children"].append(b["children"]);
//     if let Value::Object(a) = a {
//         if let Value::Object(b) = b {
//             if let Some(array) = a.as_object_mut().unwrap().get_mut("children").and_then(|v| v.as_array_mut()){
//                 array.push(b["children"]);
        

//             }
        
//         }
//     }
//     println!("{:#?}", a);
// }
impl <'ast>Visitor<'ast> for Scanner {
	type Params = AstParams<Context, ()>;
	fn object(&mut self) -> &mut dyn Visitor<'ast, Params = Self::Params> {
        self
	}
    fn visit_program(
        &mut self,
        nc: &mut Context,
        p: &'ast aast::Program<(), ()>,
    ) -> Result<(), ()> {
        let res = match p.recurse(nc, self){
            Ok(res) => res,
            Err(err) => {
                return Err(err);
            }
        };

        let obj = json!({
            "kind": "Program",
            "child": self.tree_stack.pop(),
        });

        self.tree = serde_json::to_string_pretty(&obj).unwrap();
        // println!("{}", serde_json::to_string_pretty(&obj).unwrap());
        Ok(Default::default())
    }
	fn visit_fun_def(
        &mut self,
        c: &mut Context,
        p: &'ast aast::FunDef<(), ()>,
    ) -> Result<(), ()> {
        println!("FunDef");
		let _res = match p.recurse(c, self){
			Ok(_res) => _res,
			Err(err) => {
				return Err(err);
			}
		};
        println!("Ret FunDef");
		let fd = json!({
				"kind": "FunDef",
				"name": p.name,
                "doc_comment": "Testing 123",
                "child": self.tree_stack.pop(),
		});
		self.tree_stack.push(fd);
		Ok(Default::default())
    }

	fn visit_fun_(
        &mut self,
        c: &mut Context,
        p: &'ast aast::Fun_<(), ()>,
    ) -> Result<(), ()> {
        println!("Fun_");
		let _res = match p.recurse(c, self){
			Ok(_res) => _res,
			Err(err) => {
				return Err(err);
			}
		};
        println!("Ret Fun");
        
        let body = self.tree_stack.pop();
        let mut param = self.tree_stack.pop().unwrap();
        while let Some(last) = self.tree_stack.last() {
            if last["kind"] != "FuncParam" {
                break;
            }
            let next_param = self.tree_stack.pop().unwrap();
            // merge_children(& mut param, next_param);
            param.merge(&next_param);
        }
		let f = json!({
				"kind": "Fun",
                "body": body,
                "params": param,
                "ret": self.tree_stack.pop(),
			});
        self.tree_stack.push(f);
		Ok(Default::default())
    }

	fn visit_func_body(
        &mut self,
        c: &mut Context,
        p: &'ast aast::FuncBody<(), ()>,
    ) -> Result<(), ()> {
        println!("Fun Body");
		let _res = match p.recurse(c, self){
			Ok(_res) => _res,
			Err(err) => {
				return Err(err);
			}
		};
        println!("Ret Fun Body");
		let fb = json!({
				"kind": "FuncBody",
				"children": self.tree_stack.pop(),
			});
        self.tree_stack.push(fb);
		Ok(Default::default())
    }

	fn visit_fun_param(
        &mut self,
        c: &mut Context,
        p: &'ast aast::FunParam<(), ()>,
    ) -> Result<(), ()> {
        println!("Fun Param");
		let _res = match p.recurse(c, self){
			Ok(_res) => _res,
			Err(err) => {
				return Err(err);
			}
		};
        println!("Ret Fun Param");
		let fp = json!({
				"kind": "FuncParam",
                "name": p.name.clone(),
				"children": self.tree_stack.pop(),
			});
		self.tree_stack.push(fp);
		// println!("{:#?}", fp);
		Ok(Default::default())
    }

	fn visit_type_hint(
        &mut self,
        c: &mut Context,
        p: &'ast aast::TypeHint<()>,
    ) -> Result<(), ()> {
        println!("Type Hint");
		let th = json!({
			"kind": "TypeHint",
			"child": *(p.1.clone().unwrap().1),
		});
        println!("Ret Type Hint");
		self.tree_stack.push(th);

		Ok(Default::default())
    }

    fn visit_expr_(
        &mut self,
        c: &mut Context,
        e: &'ast aast::Expr_<(), ()>,
    ) -> Result<(), ()> {
        let _res = match e.recurse(c, self){
			Ok(_res) => _res,
			Err(err) => {
				return Err(err);
			}
		};

        let ex = json!({
            "kind": "Expr",
            "child": self.tree_stack.pop(),
        });
        println!("Stmt");
        self.tree_stack.push(ex);
        Ok(Default::default())
    }
    

    fn visit_binop(
        &mut self,
        c: &mut Context,
        b: &'ast aast::Binop<(), ()>,
    ) -> Result<(), ()> {

        let bi = json!({
			"kind": "Binop",
			"child": b,
		});

        self.tree_stack.push(bi);
        Ok(Default::default())
    }
}


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