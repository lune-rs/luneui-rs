//! Signatures, written from the JSON types (never from source text). Every enum of the format
//! is matched with all its variants named, so a format change breaks the build and does not
//! silently drop a case from the output.

use rustdoc_types::{
    Abi, AssocItemConstraint, AssocItemConstraintKind, Attribute, Constant, DynTrait, Function,
    FunctionHeader, FunctionPointer, FunctionSignature, GenericArg, GenericArgs, GenericBound,
    GenericParamDef, GenericParamDefKind, Generics, Impl, Item, ItemEnum, MacroKind, Path,
    PreciseCapturingArg, ReprKind, StructKind, Term, TraitBoundModifier, Type, VariantKind,
    Visibility, WherePredicate,
};

use crate::world::{Fail, Res, World};

fn join<T>(items: &[T], sep: &str, f: impl Fn(&T) -> String) -> String {
    items.iter().map(f).collect::<Vec<_>>().join(sep)
}

pub(crate) fn ty(t: &Type) -> String {
    match t {
        Type::ResolvedPath(p) => path(p),
        Type::DynTrait(d) => dyn_trait(d),
        Type::Generic(n) => n.clone(),
        Type::Primitive(n) => n.clone(),
        Type::FunctionPointer(f) => fn_pointer(f),
        Type::Tuple(ts) if ts.len() == 1 => format!("({},)", ty(&ts[0])),
        Type::Tuple(ts) => format!("({})", join(ts, ", ", ty)),
        Type::Slice(inner) => format!("[{}]", ty(inner)),
        Type::Array { type_, len } => format!("[{}; {len}]", ty(type_)),
        Type::Pat {
            type_,
            __pat_unstable_do_not_use,
        } => {
            format!("{} is {__pat_unstable_do_not_use}", ty(type_))
        }
        Type::ImplTrait(bs) => format!("impl {}", bounds(bs)),
        Type::Infer => "_".to_string(),
        Type::RawPointer { is_mutable, type_ } => {
            format!(
                "*{} {}",
                if *is_mutable { "mut" } else { "const" },
                ty(type_)
            )
        }
        Type::BorrowedRef {
            lifetime,
            is_mutable,
            type_,
        } => {
            let lt = lifetime
                .as_ref()
                .map(|l| format!("{l} "))
                .unwrap_or_default();
            let m = if *is_mutable { "mut " } else { "" };
            format!("&{lt}{m}{}", ref_target(type_))
        }
        Type::QualifiedPath {
            name,
            args,
            self_type,
            trait_,
        } => {
            let a = args.as_deref().map(generic_args).unwrap_or_default();
            match trait_ {
                Some(tr) => format!("<{} as {}>::{name}{a}", ty(self_type), path(tr)),
                None => format!("{}::{name}{a}", ty(self_type)),
            }
        }
    }
}

/// The type behind a reference or pointer, in parentheses when it is a sum of bounds.
fn ref_target(t: &Type) -> String {
    match t {
        Type::DynTrait(d) if d.traits.len() + usize::from(d.lifetime.is_some()) > 1 => {
            format!("({})", ty(t))
        }
        Type::ImplTrait(bs) if bs.len() > 1 => format!("({})", ty(t)),
        _ => ty(t),
    }
}

/// A path as written, except that a leading `crate::`, `self::` or `super::` chain is dropped
/// (the last segment names the type, and the page it is on says where it lives).
fn path(p: &Path) -> String {
    let name = match p.path.split("::").next() {
        Some("crate" | "self" | "super" | "$crate") => {
            p.path.rsplit("::").next().unwrap_or(&p.path)
        }
        _ => &p.path,
    };
    format!(
        "{name}{}",
        p.args.as_deref().map(generic_args).unwrap_or_default()
    )
}

fn dyn_trait(d: &DynTrait) -> String {
    let mut parts: Vec<String> = d
        .traits
        .iter()
        .map(|pt| format!("{}{}", for_params(&pt.generic_params), path(&pt.trait_)))
        .collect();
    if let Some(l) = &d.lifetime {
        parts.push(l.clone());
    }
    format!("dyn {}", parts.join(" + "))
}

fn for_params(params: &[GenericParamDef]) -> String {
    if params.is_empty() {
        String::new()
    } else {
        format!("for<{}> ", join(params, ", ", param))
    }
}

pub(crate) fn generic_args(a: &GenericArgs) -> String {
    match a {
        GenericArgs::AngleBracketed { args, constraints } => {
            let mut parts: Vec<String> = args.iter().map(generic_arg).collect();
            parts.extend(constraints.iter().map(constraint));
            if parts.is_empty() {
                String::new()
            } else {
                format!("<{}>", parts.join(", "))
            }
        }
        GenericArgs::Parenthesized { inputs, output } => {
            let out = output
                .as_ref()
                .map(|o| format!(" -> {}", ty(o)))
                .unwrap_or_default();
            format!("({}){out}", join(inputs, ", ", ty))
        }
        GenericArgs::ReturnTypeNotation => "(..)".to_string(),
    }
}

fn generic_arg(a: &GenericArg) -> String {
    match a {
        GenericArg::Lifetime(l) => l.clone(),
        GenericArg::Type(t) => ty(t),
        GenericArg::Const(c) => constant(c),
        GenericArg::Infer => "_".to_string(),
    }
}

fn constant(c: &Constant) -> String {
    c.expr.clone()
}

fn constraint(c: &AssocItemConstraint) -> String {
    let args = c.args.as_deref().map(generic_args).unwrap_or_default();
    match &c.binding {
        AssocItemConstraintKind::Equality(t) => format!("{}{args} = {}", c.name, term(t)),
        AssocItemConstraintKind::Constraint(bs) => format!("{}{args}: {}", c.name, bounds(bs)),
    }
}

fn term(t: &Term) -> String {
    match t {
        Term::Type(t) => ty(t),
        Term::Constant(c) => constant(c),
    }
}

pub(crate) fn bound(b: &GenericBound) -> String {
    match b {
        GenericBound::TraitBound {
            trait_,
            generic_params,
            modifier,
        } => {
            let m = match modifier {
                TraitBoundModifier::None => "",
                TraitBoundModifier::Maybe => "?",
                TraitBoundModifier::MaybeConst => "[const] ",
            };
            format!("{m}{}{}", for_params(generic_params), path(trait_))
        }
        GenericBound::Outlives(l) => l.clone(),
        GenericBound::Use(args) => format!(
            "use<{}>",
            join(args, ", ", |a| match a {
                PreciseCapturingArg::Lifetime(l) => l.clone(),
                PreciseCapturingArg::Param(p) => p.clone(),
            })
        ),
    }
}

pub(crate) fn bounds(bs: &[GenericBound]) -> String {
    join(bs, " + ", bound)
}

fn param(p: &GenericParamDef) -> String {
    match &p.kind {
        GenericParamDefKind::Lifetime { outlives } => {
            if outlives.is_empty() {
                p.name.clone()
            } else {
                format!("{}: {}", p.name, outlives.join(" + "))
            }
        }
        GenericParamDefKind::Type {
            bounds: bs,
            default,
            is_synthetic: _,
        } => {
            let b = if bs.is_empty() {
                String::new()
            } else {
                format!(": {}", bounds(bs))
            };
            let d = default
                .as_ref()
                .map(|d| format!(" = {}", ty(d)))
                .unwrap_or_default();
            format!("{}{b}{d}", p.name)
        }
        GenericParamDefKind::Const { type_, default } => {
            let d = default
                .as_ref()
                .map(|d| format!(" = {d}"))
                .unwrap_or_default();
            format!("const {}: {}{d}", p.name, ty(type_))
        }
    }
}

fn is_synthetic(p: &GenericParamDef) -> bool {
    matches!(
        p.kind,
        GenericParamDefKind::Type {
            is_synthetic: true,
            ..
        }
    )
}

/// `<T: Bound, 'a>` or the empty string. `impl Trait` arguments (synthetic parameters) are left
/// out: they are written in the argument list.
pub(crate) fn generic_params(g: &Generics) -> String {
    let shown: Vec<String> = g
        .params
        .iter()
        .filter(|p| !is_synthetic(p))
        .map(param)
        .collect();
    if shown.is_empty() {
        String::new()
    } else {
        format!("<{}>", shown.join(", "))
    }
}

fn where_predicates(g: &Generics) -> Vec<String> {
    let synthetic: Vec<&str> = g
        .params
        .iter()
        .filter(|p| is_synthetic(p))
        .map(|p| p.name.as_str())
        .collect();
    let mut out = Vec::new();
    for p in &g.where_predicates {
        match p {
            WherePredicate::BoundPredicate {
                type_,
                bounds: bs,
                generic_params,
            } => {
                if let Type::Generic(n) = type_
                    && synthetic.contains(&n.as_str())
                {
                    continue;
                }
                out.push(format!(
                    "{}{}: {}",
                    for_params(generic_params),
                    ty(type_),
                    bounds(bs)
                ));
            }
            WherePredicate::LifetimePredicate { lifetime, outlives } => {
                out.push(format!("{lifetime}: {}", outlives.join(" + ")));
            }
            WherePredicate::EqPredicate { lhs, rhs } => {
                out.push(format!("{} == {}", ty(lhs), term(rhs)));
            }
        }
    }
    out
}

/// `\nwhere\n    T: Bound,\n` (every predicate on its own line) or the empty string.
pub(crate) fn where_block(g: &Generics) -> String {
    let preds = where_predicates(g);
    if preds.is_empty() {
        return String::new();
    }
    let mut s = String::from("\nwhere");
    for p in preds {
        s.push_str("\n    ");
        s.push_str(&p);
        s.push(',');
    }
    s
}

/// ` where T: Bound, U: Other` on one line, or the empty string.
pub(crate) fn where_inline(g: &Generics) -> String {
    let preds = where_predicates(g);
    if preds.is_empty() {
        String::new()
    } else {
        format!(" where {}", preds.join(", "))
    }
}

pub(crate) fn vis(v: &Visibility) -> String {
    match v {
        Visibility::Public => "pub ".to_string(),
        Visibility::Default => String::new(),
        Visibility::Crate => "pub(crate) ".to_string(),
        Visibility::Restricted { parent: _, path } => format!("pub(in {path}) "),
    }
}

fn abi(a: &Abi) -> String {
    let unwind = |u: &bool| if *u { "-unwind" } else { "" };
    match a {
        Abi::Rust => String::new(),
        Abi::C { unwind: u } => format!("extern \"C{}\" ", unwind(u)),
        Abi::Cdecl { unwind: u } => format!("extern \"cdecl{}\" ", unwind(u)),
        Abi::Stdcall { unwind: u } => format!("extern \"stdcall{}\" ", unwind(u)),
        Abi::Fastcall { unwind: u } => format!("extern \"fastcall{}\" ", unwind(u)),
        Abi::Aapcs { unwind: u } => format!("extern \"aapcs{}\" ", unwind(u)),
        Abi::Win64 { unwind: u } => format!("extern \"win64{}\" ", unwind(u)),
        Abi::SysV64 { unwind: u } => format!("extern \"sysv64{}\" ", unwind(u)),
        Abi::System { unwind: u } => format!("extern \"system{}\" ", unwind(u)),
        Abi::Other(s) => format!("extern \"{s}\" "),
    }
}

fn header(h: &FunctionHeader) -> String {
    format!(
        "{}{}{}{}",
        if h.is_const { "const " } else { "" },
        if h.is_async { "async " } else { "" },
        if h.is_unsafe { "unsafe " } else { "" },
        abi(&h.abi)
    )
}

fn fn_pointer(f: &FunctionPointer) -> String {
    let mut inputs: Vec<String> = f.sig.inputs.iter().map(|(_, t)| ty(t)).collect();
    if f.sig.is_c_variadic {
        inputs.push("...".to_string());
    }
    let out = f
        .sig
        .output
        .as_ref()
        .map(|o| format!(" -> {}", ty(o)))
        .unwrap_or_default();
    format!(
        "{}{}fn({}){out}",
        for_params(&f.generic_params),
        header(&f.header),
        inputs.join(", ")
    )
}

/// One argument of a function; a receiver is written as `self`.
fn input(name: &str, t: &Type) -> String {
    if name != "self" {
        return format!("{name}: {}", ty(t));
    }
    match t {
        Type::Generic(n) if n == "Self" => "self".to_string(),
        Type::BorrowedRef {
            lifetime,
            is_mutable,
            type_,
        } if matches!(&**type_, Type::Generic(n) if n == "Self") => {
            let lt = lifetime
                .as_ref()
                .map(|l| format!("{l} "))
                .unwrap_or_default();
            format!("&{lt}{}self", if *is_mutable { "mut " } else { "" })
        }
        _ => format!("self: {}", ty(t)),
    }
}

fn sig_parts(sig: &FunctionSignature) -> (Vec<String>, String) {
    let mut inputs: Vec<String> = sig.inputs.iter().map(|(n, t)| input(n, t)).collect();
    if sig.is_c_variadic {
        inputs.push("...".to_string());
    }
    let out = sig
        .output
        .as_ref()
        .map(|o| format!(" -> {}", ty(o)))
        .unwrap_or_default();
    (inputs, out)
}

/// A function's signature; the arguments go one per line when the line would pass 100 characters.
pub(crate) fn function(visibility: &str, name: &str, f: &Function) -> String {
    let (inputs, out) = sig_parts(&f.sig);
    let head = format!(
        "{visibility}{}fn {name}{}",
        header(&f.header),
        generic_params(&f.generics)
    );
    let where_ = where_block(&f.generics);
    let one = format!("{head}({}){out}", inputs.join(", "));
    if where_.is_empty() && one.len() <= 100 && !one.contains('\n') {
        return one;
    }
    let mut s = format!("{head}(");
    if !inputs.is_empty() {
        for i in &inputs {
            s.push_str("\n    ");
            s.push_str(i);
            s.push(',');
        }
        s.push('\n');
    }
    s.push(')');
    s.push_str(&out);
    s.push_str(&where_);
    s
}

/// `#[...]` lines for the attributes that carry meaning. `Other` is rustdoc's own debug text of
/// an attribute (`#[attr = Inline(Hint)]`), which is not a stable form, so it is left out, as is
/// `automatically_derived`.
pub(crate) fn attrs(list: &[Attribute]) -> Vec<String> {
    let mut out = Vec::new();
    for a in list {
        match a {
            Attribute::NonExhaustive => out.push("#[non_exhaustive]".to_string()),
            Attribute::MustUse { reason } => out.push(match reason {
                Some(r) => format!("#[must_use = {r:?}]"),
                None => "#[must_use]".to_string(),
            }),
            Attribute::MacroExport => out.push("#[macro_export]".to_string()),
            Attribute::ExportName(n) => out.push(format!("#[export_name = {n:?}]")),
            Attribute::LinkSection(n) => out.push(format!("#[link_section = {n:?}]")),
            Attribute::AutomaticallyDerived => {}
            Attribute::Repr(r) => {
                let mut parts = Vec::new();
                match r.kind {
                    ReprKind::Rust => {}
                    ReprKind::C => parts.push("C".to_string()),
                    ReprKind::Transparent => parts.push("transparent".to_string()),
                    ReprKind::Simd => parts.push("simd".to_string()),
                }
                if let Some(a) = r.align {
                    parts.push(format!("align({a})"));
                }
                if let Some(p) = r.packed {
                    parts.push(format!("packed({p})"));
                }
                if let Some(i) = &r.int {
                    parts.push(i.clone());
                }
                if !parts.is_empty() {
                    out.push(format!("#[repr({})]", parts.join(", ")));
                }
            }
            Attribute::NoMangle => out.push("#[no_mangle]".to_string()),
            Attribute::TargetFeature { enable } => {
                out.push(format!(
                    "#[target_feature(enable = {:?})]",
                    enable.join(",")
                ));
            }
            Attribute::Other(_) => {}
        }
    }
    out
}

/// ` = value` for a constant, or the empty string. A literal is shown as written; another
/// constant shows rustdoc's evaluated value when it has one. A long value, one that spans lines,
/// and one rustdoc could not print (`_`) are left to the source line.
fn const_value(c: &Constant) -> String {
    let v = if c.is_literal {
        Some(c.expr.as_str())
    } else {
        c.value.as_deref()
    };
    match v {
        Some(v) if v != "_" && v.len() <= 120 && !v.contains('\n') => format!(" = {v}"),
        _ => String::new(),
    }
}

fn field_line(w: &World, id: (usize, rustdoc_types::Id)) -> Res<String> {
    let it = w.item(id)?;
    let ItemEnum::StructField(t) = &it.inner else {
        return Err(Fail::one(format!(
            "{}: a field list names a non-field",
            w.crates[id.0].lib
        )));
    };
    Ok(format!(
        "{}{}: {}",
        vis(&it.visibility),
        it.name.as_deref().unwrap_or("?"),
        ty(t)
    ))
}

fn tuple_fields(w: &World, cn: usize, ids: &[Option<rustdoc_types::Id>]) -> Res<String> {
    let mut parts = Vec::new();
    for id in ids {
        match id {
            None => parts.push("/* private */".to_string()),
            Some(id) => {
                let it = w.item((cn, *id))?;
                let ItemEnum::StructField(t) = &it.inner else {
                    return Err(Fail::one("a tuple field list names a non-field"));
                };
                parts.push(format!("{}{}", vis(&it.visibility), ty(t)));
            }
        }
    }
    Ok(parts.join(", "))
}

fn braced_fields(
    w: &World,
    cn: usize,
    fields: &[rustdoc_types::Id],
    stripped: bool,
    indent: &str,
) -> Res<String> {
    let mut s = String::new();
    for f in fields {
        s.push_str(&format!("\n{indent}{},", field_line(w, (cn, *f))?));
    }
    if stripped {
        s.push_str(&format!("\n{indent}// some fields omitted"));
    }
    Ok(s)
}

/// The signature block of an item, written with `name` (the name it has where it is shown).
/// Items that never get a block of their own (a module, a `use`, an impl) are an error here.
pub(crate) fn signature(w: &World, cn: usize, it: &Item, name: &str) -> Res<String> {
    let v = vis(&it.visibility);
    Ok(match &it.inner {
        ItemEnum::Module(_) => return Err(Fail::one("a module has a page, not a signature")),
        ItemEnum::ExternCrate { name: n, rename } => match rename {
            Some(r) => format!("{v}extern crate {n} as {r};"),
            None => format!("{v}extern crate {n};"),
        },
        ItemEnum::Use(_) => return Err(Fail::one("a `use` has a row, not a signature")),
        ItemEnum::Union(u) => {
            let body = braced_fields(w, cn, &u.fields, u.has_stripped_fields, "    ")?;
            format!(
                "{v}union {name}{}{} {{{body}\n}}",
                generic_params(&u.generics),
                where_block(&u.generics)
            )
        }
        ItemEnum::Struct(s) => {
            let head = format!("{v}struct {name}{}", generic_params(&s.generics));
            let wb = where_block(&s.generics);
            match &s.kind {
                StructKind::Unit => format!("{head}{wb};"),
                StructKind::Tuple(ids) => format!("{head}({}){wb};", tuple_fields(w, cn, ids)?),
                StructKind::Plain {
                    fields,
                    has_stripped_fields,
                } => {
                    let body = braced_fields(w, cn, fields, *has_stripped_fields, "    ")?;
                    if body.is_empty() {
                        format!("{head}{wb} {{}}")
                    } else if wb.is_empty() {
                        format!("{head} {{{body}\n}}")
                    } else {
                        format!("{head}{wb}\n{{{body}\n}}")
                    }
                }
            }
        }
        ItemEnum::StructField(t) => format!("{v}{name}: {}", ty(t)),
        ItemEnum::Enum(e) => {
            let mut body = String::new();
            for id in &e.variants {
                body.push_str(&format!(
                    "\n    {},",
                    signature(w, cn, w.item((cn, *id))?, "")?
                ));
            }
            if e.has_stripped_variants {
                body.push_str("\n    // some variants omitted");
            }
            let head = format!("{v}enum {name}{}", generic_params(&e.generics));
            let wb = where_block(&e.generics);
            if wb.is_empty() {
                format!("{head} {{{body}\n}}")
            } else {
                format!("{head}{wb}\n{{{body}\n}}")
            }
        }
        ItemEnum::Variant(var) => {
            let vname = it.name.as_deref().unwrap_or("?");
            let mut s = vname.to_string();
            match &var.kind {
                VariantKind::Plain => {}
                VariantKind::Tuple(ids) => s.push_str(&format!("({})", tuple_fields(w, cn, ids)?)),
                VariantKind::Struct {
                    fields,
                    has_stripped_fields,
                } => {
                    let mut parts = Vec::new();
                    for f in fields {
                        parts.push(field_line(w, (cn, *f))?);
                    }
                    if *has_stripped_fields {
                        parts.push("..".to_string());
                    }
                    s.push_str(&format!(" {{ {} }}", parts.join(", ")));
                }
            }
            if let Some(d) = &var.discriminant {
                s.push_str(&format!(" = {}", d.expr));
            }
            s
        }
        ItemEnum::Function(f) => function(&v, name, f),
        ItemEnum::Trait(t) => {
            let sup = if t.bounds.is_empty() {
                String::new()
            } else {
                format!(": {}", bounds(&t.bounds))
            };
            format!(
                "{v}{}{}trait {name}{}{sup}{} {{ .. }}",
                if t.is_unsafe { "unsafe " } else { "" },
                if t.is_auto { "auto " } else { "" },
                generic_params(&t.generics),
                where_block(&t.generics)
            )
        }
        ItemEnum::TraitAlias(a) => format!(
            "{v}trait {name}{} = {}{};",
            generic_params(&a.generics),
            bounds(&a.params),
            where_block(&a.generics)
        ),
        ItemEnum::Impl(i) => impl_header(i, true),
        ItemEnum::TypeAlias(a) => format!(
            "{v}type {name}{} = {}{};",
            generic_params(&a.generics),
            ty(&a.type_),
            where_block(&a.generics)
        ),
        ItemEnum::Constant { type_, const_ } => {
            format!("{v}const {name}: {}{};", ty(type_), const_value(const_))
        }
        ItemEnum::Static(s) => {
            let value = if s.expr == "_" || s.expr.len() > 120 || s.expr.contains('\n') {
                String::new()
            } else {
                format!(" = {}", s.expr)
            };
            format!(
                "{v}static {}{name}: {}{value};",
                if s.is_mutable { "mut " } else { "" },
                ty(&s.type_)
            )
        }
        ItemEnum::ExternType => format!("{v}extern type {name};"),
        ItemEnum::Macro(src) => src.clone(),
        ItemEnum::ProcMacro(p) => match p.kind {
            MacroKind::Bang => {
                format!("#[proc_macro]\n{v}fn {name}(input: TokenStream) -> TokenStream")
            }
            MacroKind::Attr => {
                format!(
                    "#[proc_macro_attribute]\n{v}fn {name}(attr: TokenStream, item: TokenStream) -> TokenStream"
                )
            }
            MacroKind::Derive => format!(
                "#[proc_macro_derive({name}{})]\n{v}fn {name}(input: TokenStream) -> TokenStream",
                if p.helpers.is_empty() {
                    String::new()
                } else {
                    format!(", attributes({})", p.helpers.join(", "))
                }
            ),
        },
        ItemEnum::Primitive(p) => format!("primitive type {}", p.name),
        ItemEnum::AssocConst { type_, value } => {
            let val = value
                .as_ref()
                .map(|x| format!(" = {x}"))
                .unwrap_or_default();
            format!("const {name}: {}{val};", ty(type_))
        }
        ItemEnum::AssocType {
            generics,
            bounds: bs,
            type_,
        } => {
            let b = if bs.is_empty() {
                String::new()
            } else {
                format!(": {}", bounds(bs))
            };
            let d = type_
                .as_ref()
                .map(|t| format!(" = {}", ty(t)))
                .unwrap_or_default();
            format!(
                "type {name}{}{b}{d}{};",
                generic_params(generics),
                where_block(generics)
            )
        }
    })
}

/// `impl<T> Trait for Type` (with its `where` on the same line when `with_where`).
pub(crate) fn impl_header(i: &Impl, with_where: bool) -> String {
    let tr = match &i.trait_ {
        Some(t) => format!("{}{} for ", if i.is_negative { "!" } else { "" }, path(t)),
        None => String::new(),
    };
    let w = if with_where {
        where_inline(&i.generics)
    } else {
        String::new()
    };
    format!(
        "{}impl{} {tr}{}{w}",
        if i.is_unsafe { "unsafe " } else { "" },
        generic_params(&i.generics),
        ty(&i.for_)
    )
}

#[cfg(test)]
mod tests {
    use rustdoc_types::{Id, Path};

    use super::*;

    fn p(path: &str) -> Type {
        Type::ResolvedPath(Path {
            path: path.to_string(),
            id: Id(0),
            args: None,
        })
    }

    #[test]
    fn types_print_as_written() {
        assert_eq!(ty(&Type::Primitive("u8".into())), "u8");
        let r = Type::BorrowedRef {
            lifetime: Some("'a".into()),
            is_mutable: true,
            type_: Box::new(Type::Slice(Box::new(p("Foo")))),
        };
        assert_eq!(ty(&r), "&'a mut [Foo]");
        assert_eq!(ty(&Type::Tuple(vec![p("A")])), "(A,)");
        assert_eq!(ty(&Type::Tuple(vec![p("A"), p("B")])), "(A, B)");
        let arr = Type::Array {
            type_: Box::new(Type::Primitive("u8".into())),
            len: "4".into(),
        };
        assert_eq!(ty(&arr), "[u8; 4]");
    }

    #[test]
    fn a_receiver_is_written_as_self() {
        let s = Type::Generic("Self".into());
        assert_eq!(input("self", &s), "self");
        let r = Type::BorrowedRef {
            lifetime: None,
            is_mutable: true,
            type_: Box::new(s),
        };
        assert_eq!(input("self", &r), "&mut self");
        assert_eq!(input("x", &Type::Primitive("i32".into())), "x: i32");
    }
}
