use std::{collections::HashMap, ops::Neg as _};

use ark_ff::{One as _, Zero};

use crate::{
    circuit_writer::{CircuitWriter, Constant, GateKind, Var},
    constants::Span,
    error::{Error, ErrorKind, Result},
    field::Field,
    imports::FuncType,
    lexer::Token,
    parser::{FunctionSig, Ident, ParserCtx, Path},
};

use self::crypto::CRYPTO_FNS;

pub mod crypto;

#[derive(Clone)]
pub struct ImportedModule {
    pub name: String,
    pub functions: HashMap<String, (FunctionSig, FuncType)>,
    pub span: Span,
}

impl std::fmt::Debug for ImportedModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ImportedModule {{ name: {:?}, functions: {:?}, span: {:?} }}",
            self.name,
            self.functions.keys(),
            self.span
        )
    }
}

/// Parses the rest of a `use std::` statement. Returns a list of functions to import in the scope.
pub fn parse_std_import<'a>(
    path: &'a Path,
    path_iter: &mut impl Iterator<Item = &'a Ident>,
) -> Result<ImportedModule> {
    let module = path_iter.next().ok_or(Error {
        kind: ErrorKind::StdImport("no module name found"),
        span: path.span,
    })?;

    let mut res = ImportedModule {
        name: module.value.clone(),
        functions: HashMap::new(),
        span: module.span,
    };

    // TODO: make sure we're not importing the same module twice no?
    match module.value.as_ref() {
        "crypto" => {
            let crypto_functions = parse_fn_sigs(&CRYPTO_FNS);
            for func in crypto_functions {
                res.functions.insert(func.0.name.value.clone(), func);
            }
        }
        _ => {
            return Err(Error {
                kind: ErrorKind::StdImport("unknown module"),
                span: module.span,
            })
        }
    }

    Ok(res)
}

/// Takes a list of function signatures (as strings) and their associated function pointer,
/// returns the same list but with the parsed functions (as [FunctionSig]).
pub fn parse_fn_sigs(fn_sigs: &[(&str, FuncType)]) -> Vec<(FunctionSig, FuncType)> {
    let mut functions: Vec<(FunctionSig, FuncType)> = vec![];
    let ctx = &mut ParserCtx::default();

    for (sig, fn_ptr) in fn_sigs {
        let mut tokens = Token::parse(sig).unwrap();

        let sig = FunctionSig::parse(ctx, &mut tokens).unwrap();

        functions.push((sig, *fn_ptr));
    }

    functions
}

//
// Builtins or utils (imported by default)
// TODO: give a name that's useful for the user,
//       not something descriptive internally like "builtins"

const ASSERT_FN: &str = "assert(condition: Bool)";
const ASSERT_EQ_FN: &str = "assert_eq(a: Field, b: Field)";

pub const BUILTIN_FNS: [(&str, FuncType); 2] = [(ASSERT_EQ_FN, assert_eq), (ASSERT_FN, assert)];

/// Asserts that two field elements are equal.
// TODO: For now this only works for two field elements, but we could generalize that function and just divide vars into two and trust the type checker
fn assert_eq(compiler: &mut CircuitWriter, vars: &[Var], span: Span) -> Option<Var> {
    // double check (on top of type checker)
    assert_eq!(vars.len(), 2);

    match (&vars[0], &vars[1]) {
        (Var::Constant(Constant { value: a, .. }), Var::Constant(Constant { value: b, .. })) => {
            if a != b {
                panic!("assertion failed: {} != {} (TODO: return an error)", a, b);
            }
        }
        (Var::Constant(cst), Var::CircuitVar(cvars))
        | (Var::CircuitVar(cvars), Var::Constant(cst)) => {
            let cst_var = compiler.add_constant(cst.value, cst.span);

            assert_eq!(cvars.vars.len(), 1);
            let cvar = cvars.var(0).unwrap();

            // TODO: use permutation to check that
            compiler.add_gate(
                "constrain cst - var = 0 to check equality",
                GateKind::DoubleGeneric,
                vec![Some(cst_var), Some(cvar)],
                vec![Field::one(), Field::one().neg()],
                span,
            );
        }
        (Var::CircuitVar(lhs), Var::CircuitVar(rhs)) => {
            assert_eq!(lhs.vars.len(), 1);
            let lhs = lhs.var(0).unwrap();

            assert_eq!(rhs.vars.len(), 1);
            let rhs = rhs.var(0).unwrap();

            // TODO: use permutation to check that
            compiler.add_gate(
                "constrain lhs - rhs = 0 to assert that they are equal",
                GateKind::DoubleGeneric,
                vec![Some(lhs), Some(rhs)],
                vec![Field::one(), Field::one().neg()],
                span,
            );
        }
    }

    None
}

/// Asserts that a condition is true.
fn assert(compiler: &mut CircuitWriter, vars: &[Var], span: Span) -> Option<Var> {
    // double check (on top of type checker)
    assert_eq!(vars.len(), 1);

    match &vars[0] {
        Var::Constant(Constant { value: a, .. }) => {
            assert!(a.is_one());
        }
        Var::CircuitVar(cvars) => {
            assert_eq!(cvars.vars.len(), 1);
            let cvar = cvars.var(0).unwrap();

            // TODO: use permutation to check that
            let zero = Field::zero();
            let one = Field::one();
            compiler.add_gate(
                "constrain 1 - X = 0 to assert that X is true",
                GateKind::DoubleGeneric,
                vec![None, Some(cvar)],
                // use the constant to constrain 1 - X = 0
                vec![zero, one.neg(), zero, zero, one],
                span,
            );
        }
    }

    None
}
