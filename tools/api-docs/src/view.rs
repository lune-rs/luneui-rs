//! What a module exports: its own items plus what its `pub use` lines bring, with the shadowing
//! rule of Rust (an explicit name beats a glob).

use std::collections::BTreeMap;

use rustdoc_types::{ItemEnum, MacroKind};

use crate::world::{Fail, ItemRef, Res, World};

/// The kinds of item a module can hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Kind {
    Module,
    ExternCrate,
    Struct,
    Union,
    Enum,
    Trait,
    TraitAlias,
    TypeAlias,
    Function,
    Constant,
    Static,
    ExternType,
    Macro,
    AttrMacro,
    DeriveMacro,
    Primitive,
}

/// Rust's namespaces: two names clash only inside one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Ns {
    Type,
    Value,
    Macro,
}

impl Kind {
    pub(crate) fn ns(self) -> Ns {
        match self {
            Kind::Module
            | Kind::ExternCrate
            | Kind::Struct
            | Kind::Union
            | Kind::Enum
            | Kind::Trait
            | Kind::TraitAlias
            | Kind::TypeAlias
            | Kind::ExternType
            | Kind::Primitive => Ns::Type,
            Kind::Function | Kind::Constant | Kind::Static => Ns::Value,
            Kind::Macro | Kind::AttrMacro | Kind::DeriveMacro => Ns::Macro,
        }
    }

    /// The word rustdoc puts in a file name (`struct.Foo.html`) and this reference in an anchor.
    pub(crate) fn html(self) -> &'static str {
        match self {
            Kind::Module => "mod",
            Kind::ExternCrate => "externcrate",
            Kind::Struct => "struct",
            Kind::Union => "union",
            Kind::Enum => "enum",
            Kind::Trait => "trait",
            Kind::TraitAlias => "traitalias",
            Kind::TypeAlias => "type",
            Kind::Function => "fn",
            Kind::Constant => "constant",
            Kind::Static => "static",
            Kind::ExternType => "foreigntype",
            Kind::Macro => "macro",
            Kind::AttrMacro => "attr",
            Kind::DeriveMacro => "derive",
            Kind::Primitive => "primitive",
        }
    }

    /// Heading word of one item.
    pub(crate) fn word(self) -> &'static str {
        match self {
            Kind::Module => "Module",
            Kind::ExternCrate => "Extern crate",
            Kind::Struct => "Struct",
            Kind::Union => "Union",
            Kind::Enum => "Enum",
            Kind::Trait => "Trait",
            Kind::TraitAlias => "Trait alias",
            Kind::TypeAlias => "Type alias",
            Kind::Function => "Function",
            Kind::Constant => "Constant",
            Kind::Static => "Static",
            Kind::ExternType => "Extern type",
            Kind::Macro => "Macro",
            Kind::AttrMacro => "Attribute macro",
            Kind::DeriveMacro => "Derive macro",
            Kind::Primitive => "Primitive",
        }
    }

    /// Heading of a group of items on a module page; the order of this list is the page order.
    pub(crate) fn plural(self) -> &'static str {
        match self {
            Kind::Module => "Modules",
            Kind::ExternCrate => "Extern crates",
            Kind::Struct => "Structs",
            Kind::Union => "Unions",
            Kind::Enum => "Enums",
            Kind::Trait => "Traits",
            Kind::TraitAlias => "Trait aliases",
            Kind::TypeAlias => "Type aliases",
            Kind::Function => "Functions",
            Kind::Constant => "Constants",
            Kind::Static => "Statics",
            Kind::ExternType => "Extern types",
            Kind::Macro => "Macros",
            Kind::AttrMacro => "Attribute macros",
            Kind::DeriveMacro => "Derive macros",
            Kind::Primitive => "Primitives",
        }
    }
}

/// The page order of the kinds.
pub(crate) const KIND_ORDER: [Kind; 16] = [
    Kind::Module,
    Kind::Struct,
    Kind::Enum,
    Kind::Union,
    Kind::Trait,
    Kind::TraitAlias,
    Kind::TypeAlias,
    Kind::Function,
    Kind::Constant,
    Kind::Static,
    Kind::Macro,
    Kind::AttrMacro,
    Kind::DeriveMacro,
    Kind::Primitive,
    Kind::ExternType,
    Kind::ExternCrate,
];

/// The kind of an item that lives directly in a module; `None` for what lives inside another
/// item (fields, variants, impls, associated items) or is a `use`. Every variant is named, so a
/// new one in a later format breaks the build.
pub(crate) fn top_kind(inner: &ItemEnum) -> Option<Kind> {
    match inner {
        ItemEnum::Module(_) => Some(Kind::Module),
        ItemEnum::ExternCrate { .. } => Some(Kind::ExternCrate),
        ItemEnum::Use(_) => None,
        ItemEnum::Union(_) => Some(Kind::Union),
        ItemEnum::Struct(_) => Some(Kind::Struct),
        ItemEnum::StructField(_) => None,
        ItemEnum::Enum(_) => Some(Kind::Enum),
        ItemEnum::Variant(_) => None,
        ItemEnum::Function(_) => Some(Kind::Function),
        ItemEnum::Trait(_) => Some(Kind::Trait),
        ItemEnum::TraitAlias(_) => Some(Kind::TraitAlias),
        ItemEnum::Impl(_) => None,
        ItemEnum::TypeAlias(_) => Some(Kind::TypeAlias),
        ItemEnum::Constant { .. } => Some(Kind::Constant),
        ItemEnum::Static(_) => Some(Kind::Static),
        ItemEnum::ExternType => Some(Kind::ExternType),
        ItemEnum::Macro(_) => Some(Kind::Macro),
        ItemEnum::ProcMacro(p) => Some(match p.kind {
            MacroKind::Bang => Kind::Macro,
            MacroKind::Attr => Kind::AttrMacro,
            MacroKind::Derive => Kind::DeriveMacro,
        }),
        ItemEnum::Primitive(_) => Some(Kind::Primitive),
        ItemEnum::AssocConst { .. } => None,
        ItemEnum::AssocType { .. } => None,
    }
}

/// One name a module exports.
#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub(crate) name: String,
    pub(crate) kind: Kind,
    /// The item the name stands for; for a re-export, the item at its definition.
    pub(crate) target: ItemRef,
    /// True when the name comes from a `pub use` (explicit or glob), false when the module
    /// defines the item itself.
    pub(crate) used: bool,
}

pub(crate) type View = BTreeMap<(Ns, String), Entry>;

/// The names `m` (a module of any loaded crate) exports, sorted by namespace and name.
/// `stack` holds the modules being expanded, which cuts a glob cycle.
pub(crate) fn view(w: &World, m: ItemRef, stack: &mut Vec<ItemRef>) -> Res<View> {
    let item = w.item(m)?;
    let ItemEnum::Module(module) = &item.inner else {
        return Err(Fail::one(format!(
            "{}: item {} is not a module",
            w.crates[m.0].lib, m.1.0
        )));
    };
    stack.push(m);
    let mut explicit: View = BTreeMap::new();
    let mut globbed: View = BTreeMap::new();
    let mut ambiguous: Vec<((Ns, String), String)> = Vec::new();
    for id in &module.items {
        let it = w.item((m.0, *id))?;
        let ItemEnum::Use(u) = &it.inner else {
            let Some(kind) = top_kind(&it.inner) else {
                return Err(Fail::one(format!(
                    "{}: module {} holds an item that is not a module member (id {})",
                    w.crates[m.0].lib,
                    item.name.as_deref().unwrap_or("?"),
                    id.0
                )));
            };
            let name = it.name.clone().ok_or_else(|| {
                Fail::one(format!(
                    "{}: unnamed module member (id {})",
                    w.crates[m.0].lib, id.0
                ))
            })?;
            let entry = Entry {
                name: name.clone(),
                kind,
                target: (m.0, *id),
                used: false,
            };
            if explicit.insert((kind.ns(), name.clone()), entry).is_some() {
                return Err(Fail::one(format!(
                    "{}: two items named {name} in one namespace of a module",
                    w.crates[m.0].lib
                )));
            }
            continue;
        };
        let target = u.id.and_then(|tid| w.locate(m.0, tid)).ok_or_else(|| {
            Fail::one(format!(
                "{}: `pub use {}` names an item outside the five crates or one the JSON does \
                     not resolve; the reference cannot link it",
                w.crates[m.0].lib, u.source
            ))
        })?;
        let tkind = top_kind(&w.item(target)?.inner).ok_or_else(|| {
            Fail::one(format!(
                "{}: `pub use {}` names something that is not a module-level item",
                w.crates[m.0].lib, u.source
            ))
        })?;
        if u.is_glob {
            if tkind != Kind::Module {
                return Err(Fail::one(format!(
                    "{}: `pub use {}` is a glob of something other than a module",
                    w.crates[m.0].lib, u.source
                )));
            }
            if stack.contains(&target) {
                continue;
            }
            for (key, e) in view(w, target, stack)? {
                let e = Entry { used: true, ..e };
                match globbed.get(&key) {
                    Some(prev) if prev.target != e.target => ambiguous.push((
                        key.clone(),
                        format!(
                            "{}: glob re-exports bring two different items named {}",
                            w.crates[m.0].lib, key.1
                        ),
                    )),
                    Some(_) => {}
                    None => {
                        globbed.insert(key, e);
                    }
                }
            }
        } else {
            let entry = Entry {
                name: u.name.clone(),
                kind: tkind,
                target,
                used: true,
            };
            if explicit
                .insert((tkind.ns(), u.name.clone()), entry)
                .is_some()
            {
                return Err(Fail::one(format!(
                    "{}: two items named {} in one namespace of a module",
                    w.crates[m.0].lib, u.name
                )));
            }
        }
    }
    stack.pop();
    let explicit_names: std::collections::BTreeSet<(Ns, String)> =
        explicit.keys().cloned().collect();
    for (key, e) in globbed {
        explicit.entry(key).or_insert(e);
    }
    // A name two globs disagree on is an error only when no explicit item shadows it.
    if let Some((_, message)) = ambiguous
        .into_iter()
        .find(|(key, _)| !explicit_names.contains(key))
    {
        return Err(Fail::one(message));
    }
    Ok(explicit)
}
