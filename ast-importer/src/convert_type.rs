use c_ast::*;
use syntax::ast::*;
use syntax::abi::Abi;
use idiomize::ast_manip::make_ast::*;
use syntax::ptr::P;
use std::ops::Index;
use renamer::*;
use std::collections::HashSet;
use c_ast::CDeclId;

pub struct TypeConverter {
    renamer: Renamer<CDeclId>,
}

impl TypeConverter {

    pub fn new() -> TypeConverter {
        TypeConverter {
            renamer: Renamer::new(HashSet::new()),
        }
    }

    pub fn declare_decl_name(&mut self, decl_id: CDeclId, name: &str) -> String {
        self.renamer.insert(decl_id, name).expect("Name already assigned")
    }

    pub fn resolve_decl_name(&self, decl_id: CDeclId) -> Option<String> {
        self.renamer.get(&decl_id)
    }

    /// Convert a `C` type to a `Rust` one. For the moment, these are expected to have compatible
    /// memory layouts.
    pub fn convert(&mut self, ctxt: &TypedAstContext, ctype: CTypeId) -> Result<P<Ty>, String> {

        match ctxt.index(ctype).kind {
            CTypeKind::Void => Ok(mk().tuple_ty(vec![] as Vec<P<Ty>>)),
            CTypeKind::Bool => Ok(mk().path_ty(mk().path(vec!["bool"]))),
            CTypeKind::Short => Ok(mk().path_ty(mk().path(vec!["libc","c_short"]))),
            CTypeKind::Int => Ok(mk().path_ty(mk().path(vec!["libc","c_int"]))),
            CTypeKind::Long => Ok(mk().path_ty(mk().path(vec!["libc","c_long"]))),
            CTypeKind::LongLong => Ok(mk().path_ty(mk().path(vec!["libc","c_longlong"]))),
            CTypeKind::UShort => Ok(mk().path_ty(mk().path(vec!["libc","c_ushort"]))),
            CTypeKind::UInt => Ok(mk().path_ty(mk().path(vec!["libc","c_uint"]))),
            CTypeKind::ULong => Ok(mk().path_ty(mk().path(vec!["libc","c_ulong"]))),
            CTypeKind::ULongLong => Ok(mk().path_ty(mk().path(vec!["libc","c_ulonglong"]))),
            CTypeKind::SChar => Ok(mk().path_ty(mk().path(vec!["libc","c_schar"]))),
            CTypeKind::UChar => Ok(mk().path_ty(mk().path(vec!["libc","c_uchar"]))),
            CTypeKind::Char => Ok(mk().path_ty(mk().path(vec!["libc","c_char"]))),
            CTypeKind::Double => Ok(mk().path_ty(mk().path(vec!["libc","c_double"]))),
            CTypeKind::Float => Ok(mk().path_ty(mk().path(vec!["libc","c_float"]))),
            CTypeKind::Int128 => Ok(mk().path_ty(mk().path(vec!["i128"]))),
            CTypeKind::UInt128 => Ok(mk().path_ty(mk().path(vec!["u128"]))),

            CTypeKind::Pointer(CQualTypeId { ref qualifiers, ref ctype }) => {
                match ctxt.resolve_type(*ctype).kind {

                    // While void converts to () in function returns, it converts to c_void
                    // in the case of pointers.
                    CTypeKind::Void => {
                            let mutbl = if qualifiers.is_const { Mutability::Immutable } else { Mutability::Mutable };
                            Ok(mk().set_mutbl(mutbl).ptr_ty(mk().path_ty(vec!["libc","c_void"])))
                    }

                    // Function pointers are translated to Option applied to the function type
                    // in order to support NULL function pointers natively
                    CTypeKind::Function(ref ret, ref params) => {
                        let inputs = params.iter().map(|x|
                            mk().arg(self.convert(ctxt, x.ctype).unwrap(),
                                     mk().wild_pat())
                        ).collect();
                        let output = self.convert(ctxt, ret.ctype)?;
                        let fn_ptr = mk().unsafe_().abi(Abi::C).barefn_ty(mk().fn_decl(inputs, FunctionRetTy::Ty(output)));
                        let param = mk().angle_bracketed_param_types(vec![fn_ptr]);
                        Ok(mk().path_ty(vec![mk().path_segment_with_params("Option", param)]))
                    }

                    _ => {
                        let child_ty = self.convert(ctxt, *ctype)?;
                        let mutbl = if qualifiers.is_const { Mutability::Immutable } else { Mutability::Mutable };
                        Ok(mk().set_mutbl(mutbl).ptr_ty(child_ty))
                    }
                }
            }

            CTypeKind::Elaborated(ref ctype) => self.convert(ctxt, *ctype),
            CTypeKind::Decayed(ref ctype) => self.convert(ctxt, *ctype),
            CTypeKind::Paren(ref ctype) => self.convert(ctxt, *ctype),

            CTypeKind::Struct(decl_id) => {
                let new_name = self.resolve_decl_name(decl_id).ok_or_else(|| format!("Unknown decl id {:?}", decl_id))?;
                Ok(mk().path_ty(mk().path(vec![new_name])))
            }

            CTypeKind::Union(decl_id) => {
                let new_name = self.resolve_decl_name(decl_id).unwrap();
                Ok(mk().path_ty(mk().path(vec![new_name])))
            }

            CTypeKind::Enum(decl_id) => {
                let new_name = self.resolve_decl_name(decl_id).unwrap();
                Ok(mk().path_ty(mk().path(vec![new_name])))
            }

            CTypeKind::Typedef(decl_id) => {
                let new_name = self.resolve_decl_name(decl_id).unwrap();
                Ok(mk().path_ty(mk().path(vec![new_name])))
            }

            CTypeKind::ConstantArray(element, count) => {
                let ty = self.convert(ctxt, element)?;
                Ok(mk().array_ty(ty, mk().lit_expr(mk().int_lit(count as u128, LitIntType::Unsuffixed))))
            }

            CTypeKind::Attributed(ty) => self.convert(ctxt, ty.ctype),

            ref t => Err(format!("Unsupported type {:?}", t)),
        }
    }
}
