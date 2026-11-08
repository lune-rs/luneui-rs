//! Which page documents what: the module tree of each crate as rustdoc shows it, where each
//! item's body goes, which re-exports are only rows, and the anchor of every item and member.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use rustdoc_types::{Impl, Item, ItemEnum, StructKind, VariantKind};

use crate::view::{KIND_ORDER, Kind, view};
use crate::world::{ItemRef, Res, World};

/// A page: the crate and the module path under it (empty for the crate root).
pub(crate) type PageKey = (usize, Vec<String>);

/// A re-export shown as a row on a page instead of a body.
#[derive(Debug)]
pub(crate) struct Row {
    pub(crate) name: String,
    pub(crate) kind: Kind,
    pub(crate) target: ItemRef,
}

/// An item whose body is on a page.
#[derive(Debug)]
pub(crate) struct Placed {
    pub(crate) name: String,
    pub(crate) kind: Kind,
    pub(crate) item: ItemRef,
}

#[derive(Debug)]
pub(crate) struct Page {
    pub(crate) module: ItemRef,
    pub(crate) body: Vec<Placed>,
    pub(crate) rows: Vec<Row>,
    /// Names of the modules this module defines (each has a page of its own).
    pub(crate) submodules: Vec<String>,
    /// The `use` items among the module's items; each is shown as a row.
    pub(crate) uses: Vec<ItemRef>,
}

/// What an item holds that has its own entry on the page.
#[derive(Debug, Default)]
pub(crate) struct Members {
    pub(crate) fields: Vec<ItemRef>,
    pub(crate) variants: Vec<ItemRef>,
    /// Inherent impl blocks and their items (functions, constants), sorted.
    pub(crate) inherent: Vec<(ItemRef, Vec<ItemRef>)>,
    /// Trait impls shown as one line each.
    pub(crate) trait_impls: Vec<ItemRef>,
    /// Synthetic and blanket impls, omitted and counted: (impl, class).
    pub(crate) skipped_impls: Vec<(ItemRef, &'static str)>,
    /// A trait's own items.
    pub(crate) trait_items: Vec<ItemRef>,
    /// Impls of a trait on types that have no page of their own for it.
    pub(crate) implementors: Vec<ItemRef>,
}

#[derive(Debug, Default)]
pub(crate) struct Plan {
    pub(crate) pages: BTreeMap<PageKey, Page>,
    /// Per crate: the (rustdoc kind word, path) of every item rustdoc lists in `all.html`.
    pub(crate) listed: Vec<BTreeSet<(&'static str, String)>>,
    /// The page of each module that has one.
    pub(crate) module_page: HashMap<ItemRef, PageKey>,
    pub(crate) members: HashMap<ItemRef, Members>,
    /// The page and anchor of every placed item and member (modules: their page, empty anchor).
    pub(crate) anchors: HashMap<ItemRef, (PageKey, String)>,
}

struct Builder<'a> {
    w: &'a World,
    plan: &'a mut Plan,
    home: usize,
    /// Items defined in a public module of the home crate (not reached through a `use`).
    pd: HashSet<ItemRef>,
    placed: HashSet<ItemRef>,
}

pub(crate) fn build(w: &World) -> Res<Plan> {
    let mut plan = Plan::default();
    for ci in 0..w.crates.len() {
        plan.listed.push(BTreeSet::new());
        let root = (ci, w.crates[ci].data.root);
        let mut pd = HashSet::new();
        public_defs(w, root, &mut pd)?;
        let mut b = Builder {
            w,
            plan: &mut plan,
            home: ci,
            pd,
            placed: HashSet::new(),
        };
        b.walk(root, &[], true, &mut Vec::new())?;
    }
    layout(w, &mut plan)?;
    Ok(plan)
}

fn public_defs(w: &World, m: ItemRef, out: &mut HashSet<ItemRef>) -> Res<()> {
    let ItemEnum::Module(module) = &w.item(m)?.inner else {
        return Ok(());
    };
    for id in &module.items {
        let it = w.item((m.0, *id))?;
        if matches!(it.inner, ItemEnum::Use(_)) {
            continue;
        }
        out.insert((m.0, *id));
        if matches!(it.inner, ItemEnum::Module(_)) {
            public_defs(w, (m.0, *id), out)?;
        }
    }
    Ok(())
}

impl Builder<'_> {
    /// Walks module `m` at `path`. `own` is true while the module is one of the home crate's
    /// pages; below a re-exported module (another crate's, or a private one) the walk only
    /// records what rustdoc lists.
    fn walk(
        &mut self,
        m: ItemRef,
        path: &[String],
        own: bool,
        aliases: &mut Vec<ItemRef>,
    ) -> Res<()> {
        let key: PageKey = (self.home, path.to_vec());
        if own {
            let ItemEnum::Module(module) = &self.w.item(m)?.inner else {
                return Ok(());
            };
            let mut uses = Vec::new();
            for id in &module.items {
                if matches!(self.w.item((m.0, *id))?.inner, ItemEnum::Use(_)) {
                    uses.push((m.0, *id));
                }
            }
            self.plan.pages.insert(
                key.clone(),
                Page {
                    module: m,
                    body: Vec::new(),
                    rows: Vec::new(),
                    submodules: Vec::new(),
                    uses,
                },
            );
            self.plan.module_page.insert(m, key.clone());
        }
        let entries = view(self.w, m, &mut Vec::new())?;
        for e in entries.values() {
            let mut ipath = path.to_vec();
            ipath.push(e.name.clone());
            let plain = e.used && e.target.0 == self.home && self.pd.contains(&e.target);
            if e.kind == Kind::Module {
                if !e.used {
                    if own && let Some(p) = self.plan.pages.get_mut(&key) {
                        p.submodules.push(e.name.clone());
                    }
                    self.walk(e.target, &ipath, own, aliases)?;
                } else if plain {
                    if own {
                        self.row(&key, e.name.clone(), e.kind, e.target);
                    }
                } else {
                    if own {
                        self.row(&key, e.name.clone(), e.kind, e.target);
                    }
                    if !aliases.contains(&e.target) {
                        aliases.push(e.target);
                        self.walk(e.target, &ipath, false, aliases)?;
                        aliases.pop();
                    }
                }
                continue;
            }
            if plain {
                if own {
                    self.row(&key, e.name.clone(), e.kind, e.target);
                }
                continue;
            }
            self.plan.listed[self.home].insert((e.kind.html(), ipath.join("/")));
            if !own {
                continue;
            }
            if e.target.0 == self.home {
                if self.placed.insert(e.target) {
                    if let Some(p) = self.plan.pages.get_mut(&key) {
                        p.body.push(Placed {
                            name: e.name.clone(),
                            kind: e.kind,
                            item: e.target,
                        });
                    }
                } else {
                    self.row(&key, e.name.clone(), e.kind, e.target);
                }
            } else {
                self.row(&key, e.name.clone(), e.kind, e.target);
            }
        }
        Ok(())
    }

    fn row(&mut self, key: &PageKey, name: String, kind: Kind, target: ItemRef) {
        if let Some(p) = self.plan.pages.get_mut(key) {
            p.rows.push(Row { name, kind, target });
        }
    }
}

/// The name of an item, or `?`.
pub(crate) fn name_of(it: &Item) -> &str {
    it.name.as_deref().unwrap_or("?")
}

/// The impls a type lists (a primitive's too); empty for everything else. Every variant named.
fn own_impls(inner: &ItemEnum) -> &[rustdoc_types::Id] {
    match inner {
        ItemEnum::Struct(s) => &s.impls,
        ItemEnum::Enum(e) => &e.impls,
        ItemEnum::Union(u) => &u.impls,
        ItemEnum::Primitive(p) => &p.impls,
        ItemEnum::Module(_)
        | ItemEnum::ExternCrate { .. }
        | ItemEnum::Use(_)
        | ItemEnum::StructField(_)
        | ItemEnum::Variant(_)
        | ItemEnum::Function(_)
        | ItemEnum::Trait(_)
        | ItemEnum::TraitAlias(_)
        | ItemEnum::Impl(_)
        | ItemEnum::TypeAlias(_)
        | ItemEnum::Constant { .. }
        | ItemEnum::Static(_)
        | ItemEnum::ExternType
        | ItemEnum::Macro(_)
        | ItemEnum::ProcMacro(_)
        | ItemEnum::AssocConst { .. }
        | ItemEnum::AssocType { .. } => &[],
    }
}

/// Assigns anchors and finds each item's members.
fn layout(w: &World, plan: &mut Plan) -> Res<()> {
    for page in plan.pages.values_mut() {
        page.body.sort_by(|a, b| {
            let ka = KIND_ORDER.iter().position(|k| *k == a.kind);
            let kb = KIND_ORDER.iter().position(|k| *k == b.kind);
            (ka, &a.name, a.kind).cmp(&(kb, &b.name, b.kind))
        });
        page.rows
            .sort_by(|a, b| (&a.name, a.kind).cmp(&(&b.name, b.kind)));
        page.submodules.sort();
    }
    // An impl can be listed by more than one type (`impl From<&A> for B` is on A and on B). It is
    // written once, on the type it is for when that type is documented, else on the first
    // documented type that lists it.
    let mut listed_by: BTreeMap<ItemRef, Vec<ItemRef>> = BTreeMap::new();
    for page in plan.pages.values() {
        for p in &page.body {
            for i in own_impls(&w.item(p.item)?.inner) {
                listed_by.entry((p.item.0, *i)).or_default().push(p.item);
            }
        }
    }
    let mut owner: HashMap<ItemRef, ItemRef> = HashMap::new();
    for (imp, types) in &listed_by {
        let ItemEnum::Impl(i) = &w.item(*imp)?.inner else {
            continue;
        };
        let for_id = match &i.for_ {
            rustdoc_types::Type::ResolvedPath(p) => Some(p.id),
            _ => None,
        };
        let chosen = types
            .iter()
            .find(|t| Some(t.1) == for_id)
            .unwrap_or(&types[0]);
        owner.insert(*imp, *chosen);
    }
    let claimed: HashSet<ItemRef> = owner.keys().copied().collect();
    let keys: Vec<PageKey> = plan.pages.keys().cloned().collect();
    for key in keys {
        let mut used: HashSet<String> = HashSet::new();
        let Some(page) = plan.pages.get_mut(&key) else {
            continue;
        };
        let module = page.module;
        let placed: Vec<(String, Kind, ItemRef)> = page
            .body
            .iter()
            .map(|p| (p.name.clone(), p.kind, p.item))
            .collect();
        let mut anchors: Vec<(ItemRef, String)> = Vec::new();
        let mut members: Vec<(ItemRef, Members)> = Vec::new();
        for (name, kind, item) in placed {
            let base = format!("{}.{}", kind.html(), name);
            anchors.push((item, uniq(&mut used, base)));
            let m = members_of(w, item, &claimed, &owner)?;
            let pre = |k: &str| format!("{k}.{name}");
            for f in &m.fields {
                let f_name = field_name(w, *f, &m.fields)?;
                anchors.push((
                    *f,
                    uniq(&mut used, format!("{}.{f_name}", pre("structfield"))),
                ));
            }
            for v in &m.variants {
                let v_name = name_of(w.item(*v)?).to_string();
                anchors.push((*v, uniq(&mut used, format!("{}.{v_name}", pre("variant")))));
                let vf = variant_fields(w, *v)?;
                for f in &vf {
                    let f_name = field_name(w, *f, &vf)?;
                    anchors.push((
                        *f,
                        uniq(
                            &mut used,
                            format!("{}.{v_name}.{f_name}", pre("structfield")),
                        ),
                    ));
                }
            }
            for (_, items) in &m.inherent {
                for i in items {
                    let it = w.item(*i)?;
                    let k = match it.inner {
                        ItemEnum::AssocConst { .. } => "associatedconstant",
                        _ => "method",
                    };
                    anchors.push((*i, uniq(&mut used, format!("{}.{}", pre(k), name_of(it)))));
                }
            }
            for i in &m.trait_items {
                let it = w.item(*i)?;
                let k = match &it.inner {
                    ItemEnum::Function(f) if f.has_body => "method",
                    ItemEnum::Function(_) => "tymethod",
                    ItemEnum::AssocConst { .. } => "associatedconstant",
                    _ => "associatedtype",
                };
                anchors.push((*i, uniq(&mut used, format!("{}.{}", pre(k), name_of(it)))));
            }
            members.push((item, m));
        }
        plan.module_page.insert(module, key.clone());
        for (r, a) in anchors {
            plan.anchors.insert(r, (key.clone(), a));
        }
        plan.members.extend(members);
    }
    // A module's anchor is its page with no fragment.
    let mods: Vec<(ItemRef, PageKey)> = plan
        .module_page
        .iter()
        .map(|(r, k)| (*r, k.clone()))
        .collect();
    for (r, k) in mods {
        plan.anchors.insert(r, (k, String::new()));
    }
    Ok(())
}

/// `base`, or `base-2`, `base-3` when taken on the page.
fn uniq(used: &mut HashSet<String>, base: String) -> String {
    if used.insert(base.clone()) {
        return base;
    }
    let mut n = 2;
    loop {
        let cand = format!("{base}-{n}");
        if used.insert(cand.clone()) {
            return cand;
        }
        n += 1;
    }
}

/// The name of a field: its own, or its position among `siblings` for a tuple field.
pub(crate) fn field_name(w: &World, f: ItemRef, siblings: &[ItemRef]) -> Res<String> {
    let it = w.item(f)?;
    Ok(match &it.name {
        Some(n) => n.clone(),
        None => siblings
            .iter()
            .position(|s| *s == f)
            .unwrap_or(0)
            .to_string(),
    })
}

/// The fields of a variant that has them.
pub(crate) fn variant_fields(w: &World, v: ItemRef) -> Res<Vec<ItemRef>> {
    let ItemEnum::Variant(var) = &w.item(v)?.inner else {
        return Ok(Vec::new());
    };
    Ok(match &var.kind {
        VariantKind::Plain => Vec::new(),
        VariantKind::Tuple(ids) => ids.iter().flatten().map(|i| (v.0, *i)).collect(),
        VariantKind::Struct { fields, .. } => fields.iter().map(|i| (v.0, *i)).collect(),
    })
}

fn members_of(
    w: &World,
    item: ItemRef,
    claimed: &HashSet<ItemRef>,
    owner: &HashMap<ItemRef, ItemRef>,
) -> Res<Members> {
    let it = w.item(item)?;
    let mut m = Members::default();
    let cn = item.0;
    let impl_ids: &[rustdoc_types::Id] = match &it.inner {
        ItemEnum::Struct(s) => {
            m.fields = match &s.kind {
                StructKind::Unit => Vec::new(),
                StructKind::Tuple(ids) => ids.iter().flatten().map(|i| (cn, *i)).collect(),
                StructKind::Plain { fields, .. } => fields.iter().map(|i| (cn, *i)).collect(),
            };
            &s.impls
        }
        ItemEnum::Union(u) => {
            m.fields = u.fields.iter().map(|i| (cn, *i)).collect();
            &u.impls
        }
        ItemEnum::Enum(e) => {
            m.variants = e.variants.iter().map(|i| (cn, *i)).collect();
            &e.impls
        }
        ItemEnum::Primitive(p) => &p.impls,
        ItemEnum::Trait(t) => {
            m.trait_items = t.items.iter().map(|i| (cn, *i)).collect();
            m.implementors = t
                .implementations
                .iter()
                .map(|i| (cn, *i))
                .filter(|r| !claimed.contains(r))
                .collect();
            &[]
        }
        ItemEnum::Module(_)
        | ItemEnum::ExternCrate { .. }
        | ItemEnum::Use(_)
        | ItemEnum::StructField(_)
        | ItemEnum::Variant(_)
        | ItemEnum::Function(_)
        | ItemEnum::TraitAlias(_)
        | ItemEnum::Impl(_)
        | ItemEnum::TypeAlias(_)
        | ItemEnum::Constant { .. }
        | ItemEnum::Static(_)
        | ItemEnum::ExternType
        | ItemEnum::Macro(_)
        | ItemEnum::ProcMacro(_)
        | ItemEnum::AssocConst { .. }
        | ItemEnum::AssocType { .. } => &[],
    };
    for id in impl_ids {
        let r = (cn, *id);
        if owner.get(&r) != Some(&item) {
            continue;
        }
        let ItemEnum::Impl(imp) = &w.item(r)?.inner else {
            continue;
        };
        match impl_class(imp) {
            ImplClass::Synthetic => m.skipped_impls.push((r, "synthetic impls")),
            ImplClass::Blanket => m.skipped_impls.push((r, "blanket impls")),
            ImplClass::Trait => m.trait_impls.push(r),
            ImplClass::Inherent => {
                let mut items: Vec<ItemRef> = imp.items.iter().map(|i| (cn, *i)).collect();
                let mut keyed = Vec::new();
                for i in items.drain(..) {
                    keyed.push((name_of(w.item(i)?).to_string(), i));
                }
                keyed.sort_by(|a, b| a.0.cmp(&b.0));
                m.inherent
                    .push((r, keyed.into_iter().map(|(_, i)| i).collect()));
            }
        }
    }
    Ok(m)
}

pub(crate) enum ImplClass {
    Synthetic,
    Blanket,
    Trait,
    Inherent,
}

pub(crate) fn impl_class(imp: &Impl) -> ImplClass {
    if imp.is_synthetic {
        ImplClass::Synthetic
    } else if imp.blanket_impl.is_some() {
        ImplClass::Blanket
    } else if imp.trait_.is_some() {
        ImplClass::Trait
    } else {
        ImplClass::Inherent
    }
}
