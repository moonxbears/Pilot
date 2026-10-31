//! Milestone 33.1: Data-Oriented `TypeArena` & Row-Polymorphic Type Representation.

use std::collections::HashMap;

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(pub u32);

impl TypeId {
    pub const VOID: Self = Self(0);
    pub const NEVER: Self = Self(1);
    pub const ANY: Self = Self(2);
    pub const NULL: Self = Self(3);
    pub const BOOL: Self = Self(4);
    pub const I64: Self = Self(5);
    pub const I32: Self = Self(6);
    pub const F64: Self = Self(7);
    pub const F32: Self = Self(8);
    pub const STR: Self = Self(9);

    #[inline]
    pub const fn is_primitive(self) -> bool {
        self.0 <= 9
    }

    #[inline]
    pub const fn is_integer(self) -> bool {
        self.0 == Self::I64.0 || self.0 == Self::I32.0
    }

    #[inline]
    pub const fn is_float(self) -> bool {
        self.0 == Self::F64.0 || self.0 == Self::F32.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CollectionKind {
    List,
    Set,
    Future,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeKind {
    Primitive(String),
    Collection {
        kind: CollectionKind,
        element: TypeId,
    },
    Map {
        key: TypeId,
        value: TypeId,
    },
    Tuple(Vec<TypeId>),
    Option(TypeId),
    Result {
        ok: TypeId,
        err: TypeId,
    },
    Function {
        params: Vec<TypeId>,
        ret: TypeId,
    },
    Matrix {
        rows: u32,
        cols: u32,
    },
    Unit {
        base: TypeId,
        unit_name: String,
    },
    Union(Vec<TypeId>),
    TypeVar {
        id: u32,
        constraint: u32, // bitmask: 1=Numeric, 2=Comparable, 4=Equatable, 8=Addable, 16=Iterable
    },
    RowRecord {
        fields: Vec<(String, TypeId)>, // sorted by field name for canonical identity
        is_open: bool,
        tail_var: Option<u32>,
        name: Option<String>,
    },
    Typestate {
        base: TypeId,
        state_name: String,
    },
    Adt {
        name: String,
        variants: Vec<(String, Vec<TypeId>)>,
    },
    Void,
    Never,
    Any,
    Null,
}

#[derive(Debug)]
pub struct TypeArena {
    types: Vec<TypeKind>,
    type_map: HashMap<TypeKind, TypeId>,
    pub nominal_names: HashMap<TypeId, String>,
    next_var_id: u32,
    subst: HashMap<u32, TypeId>,
}

impl Default for TypeArena {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeArena {
    pub fn new() -> Self {
        let mut arena = Self {
            types: Vec::with_capacity(64),
            type_map: HashMap::with_capacity(64),
            nominal_names: HashMap::new(),
            next_var_id: 100,
            subst: HashMap::new(),
        };

        // Initialize predefined primitive types at 0..9
        arena.register_builtin(TypeId::VOID, TypeKind::Void);
        arena.register_builtin(TypeId::NEVER, TypeKind::Never);
        arena.register_builtin(TypeId::ANY, TypeKind::Any);
        arena.register_builtin(TypeId::NULL, TypeKind::Null);
        arena.register_builtin(TypeId::BOOL, TypeKind::Primitive("bool".to_string()));
        arena.register_builtin(TypeId::I64, TypeKind::Primitive("i64".to_string()));
        arena.register_builtin(TypeId::I32, TypeKind::Primitive("i32".to_string()));
        arena.register_builtin(TypeId::F64, TypeKind::Primitive("f64".to_string()));
        arena.register_builtin(TypeId::F32, TypeKind::Primitive("f32".to_string()));
        arena.register_builtin(TypeId::STR, TypeKind::Primitive("str".to_string()));

        arena
    }

    fn register_builtin(&mut self, expected_id: TypeId, kind: TypeKind) {
        let id = TypeId(self.types.len() as u32);
        assert_eq!(id, expected_id);
        self.types.push(kind.clone());
        self.type_map.insert(kind, id);
    }

    pub fn alloc(&mut self, kind: TypeKind) -> TypeId {
        if let Some(&existing) = self.type_map.get(&kind) {
            return existing;
        }

        let id = TypeId(self.types.len() as u32);
        self.types.push(kind.clone());
        self.type_map.insert(kind, id);
        id
    }

    pub fn get(&self, id: TypeId) -> &TypeKind {
        &self.types[id.0 as usize]
    }

    pub fn new_var(&mut self, constraint: u32) -> TypeId {
        let id = self.next_var_id;
        self.next_var_id += 1;
        self.alloc(TypeKind::TypeVar { id, constraint })
    }

    pub fn list_of(&mut self, elem: TypeId) -> TypeId {
        self.alloc(TypeKind::Collection {
            kind: CollectionKind::List,
            element: elem,
        })
    }

    pub fn map_of(&mut self, key: TypeId, value: TypeId) -> TypeId {
        self.alloc(TypeKind::Map { key, value })
    }

    pub fn option_of(&mut self, elem: TypeId) -> TypeId {
        if let TypeKind::Option(_) = self.get(elem) {
            return elem;
        }
        self.alloc(TypeKind::Option(elem))
    }

    pub fn result_of(&mut self, ok: TypeId, err: TypeId) -> TypeId {
        self.alloc(TypeKind::Result { ok, err })
    }

    pub fn tuple_of(&mut self, elems: Vec<TypeId>) -> TypeId {
        self.alloc(TypeKind::Tuple(elems))
    }

    pub fn function_of(&mut self, params: Vec<TypeId>, ret: TypeId) -> TypeId {
        self.alloc(TypeKind::Function { params, ret })
    }

    pub fn unit_of(&mut self, base: TypeId, unit_name: String) -> TypeId {
        self.alloc(TypeKind::Unit { base, unit_name })
    }

    pub fn typestate_of(&mut self, base: TypeId, state_name: String) -> TypeId {
        self.alloc(TypeKind::Typestate { base, state_name })
    }

    pub fn adt_of(&mut self, name: String, variants: Vec<(String, Vec<TypeId>)>) -> TypeId {
        self.alloc(TypeKind::Adt { name, variants })
    }

    pub fn register_nominal_name(&mut self, id: TypeId, name: String) {
        self.nominal_names.insert(id, name);
    }

    pub fn row_record_of(
        &mut self,
        fields: Vec<(String, TypeId)>,
        is_open: bool,
        tail_var: Option<u32>,
    ) -> TypeId {
        self.row_record_with_name(fields, is_open, tail_var, None)
    }

    pub fn row_record_with_name(
        &mut self,
        mut fields: Vec<(String, TypeId)>,
        is_open: bool,
        tail_var: Option<u32>,
        name: Option<String>,
    ) -> TypeId {
        fields.sort_by(|a, b| a.0.cmp(&b.0));
        let id = self.alloc(TypeKind::RowRecord {
            fields,
            is_open,
            tail_var,
            name: name.clone(),
        });
        if let Some(n) = name {
            self.nominal_names.insert(id, n);
        }
        id
    }

    /// Forms a canonical, flattened, and deduplicated Union type from a set of types.
    pub fn union_of(&mut self, types: Vec<TypeId>) -> TypeId {
        let mut flattened = Vec::new();
        for t in types {
            let pruned = self.prune(t);
            if pruned == TypeId::ANY {
                return TypeId::ANY;
            }
            if pruned == TypeId::NEVER || pruned == TypeId::VOID {
                continue;
            }
            if let TypeKind::Union(inner) = self.get(pruned).clone() {
                for u in inner {
                    let u_pruned = self.prune(u);
                    if !flattened.contains(&u_pruned) {
                        flattened.push(u_pruned);
                    }
                }
            } else if !flattened.contains(&pruned) {
                flattened.push(pruned);
            }
        }
        if flattened.is_empty() {
            TypeId::NEVER
        } else if flattened.len() == 1 {
            flattened[0]
        } else {
            flattened.sort_by_key(|t| t.0);
            self.alloc(TypeKind::Union(flattened))
        }
    }

    /// Unifies all types if they are mutually compatible; otherwise forms a constrained Union.
    pub fn unify_or_union_all(&mut self, types: Vec<TypeId>) -> TypeId {
        if types.is_empty() {
            return self.new_var(0);
        }
        let mut unified = self.prune(types[0]);
        let mut all_unified = true;
        for &t in &types[1..] {
            let prev_subst = self.subst.clone();
            match self.unify(unified, t) {
                Ok(u) => unified = u,
                Err(_) => {
                    self.subst = prev_subst;
                    all_unified = false;
                    break;
                }
            }
        }
        if all_unified {
            return unified;
        }
        self.union_of(types)
    }

    /// Resolves any assigned type substitutions for a given TypeId.
    pub fn prune(&self, mut id: TypeId) -> TypeId {
        while let TypeKind::TypeVar { id: var_id, .. } = self.get(id) {
            if let Some(&next) = self.subst.get(var_id) {
                id = next;
            } else {
                break;
            }
        }
        id
    }

    /// Recursively substitutes all type variables across nested composite types.
    pub fn deep_prune(&mut self, ty: TypeId) -> TypeId {
        let pruned = self.prune(ty);
        match self.get(pruned).clone() {
            TypeKind::Collection { kind, element } => {
                let e = self.deep_prune(element);
                self.alloc(TypeKind::Collection { kind, element: e })
            }
            TypeKind::Map { key, value } => {
                let k = self.deep_prune(key);
                let v = self.deep_prune(value);
                self.alloc(TypeKind::Map { key: k, value: v })
            }
            TypeKind::Tuple(elems) => {
                let pr_elems: Vec<_> = elems.into_iter().map(|e| self.deep_prune(e)).collect();
                self.alloc(TypeKind::Tuple(pr_elems))
            }
            TypeKind::Option(inner) => {
                let inn = self.deep_prune(inner);
                self.alloc(TypeKind::Option(inn))
            }
            TypeKind::Result { ok, err } => {
                let o = self.deep_prune(ok);
                let e = self.deep_prune(err);
                self.alloc(TypeKind::Result { ok: o, err: e })
            }
            TypeKind::Function { params, ret } => {
                let pr_params: Vec<_> = params.into_iter().map(|p| self.deep_prune(p)).collect();
                let pr_ret = self.deep_prune(ret);
                self.alloc(TypeKind::Function { params: pr_params, ret: pr_ret })
            }
            TypeKind::Union(types) => {
                let pr_types: Vec<_> = types.into_iter().map(|t| self.deep_prune(t)).collect();
                self.union_of(pr_types)
            }
            TypeKind::RowRecord { fields, is_open, tail_var, name } => {
                let pr_fields: Vec<_> = fields.into_iter().map(|(n, t)| (n, self.deep_prune(t))).collect();
                self.row_record_with_name(pr_fields, is_open, tail_var, name)
            }
            _ => pruned,
        }
    }

    /// Unifies two types in the arena, recording type variable substitutions.
    pub fn unify(&mut self, a: TypeId, b: TypeId) -> Result<TypeId, String> {
        let a = self.prune(a);
        let b = self.prune(b);

        if a == b {
            return Ok(a);
        }

        // ANY unifies with everything
        if a == TypeId::ANY {
            return Ok(b);
        }
        if b == TypeId::ANY {
            return Ok(a);
        }

        // Never unifies as bottom type
        if a == TypeId::NEVER {
            return Ok(b);
        }
        if b == TypeId::NEVER {
            return Ok(a);
        }

        match (self.get(a).clone(), self.get(b).clone()) {
            (TypeKind::TypeVar { id: v, .. }, _) => {
                self.subst.insert(v, b);
                Ok(b)
            }
            (_, TypeKind::TypeVar { id: v, .. }) => {
                self.subst.insert(v, a);
                Ok(a)
            }
            (TypeKind::Option(elem_a), TypeKind::Option(elem_b)) => {
                let elem = self.unify(elem_a, elem_b)?;
                Ok(self.option_of(elem))
            }
            (TypeKind::Collection { kind: k1, element: e1 }, TypeKind::Collection { kind: k2, element: e2 }) if k1 == k2 => {
                let elem = self.unify(e1, e2)?;
                Ok(self.alloc(TypeKind::Collection { kind: k1, element: elem }))
            }
            (TypeKind::Map { key: k1, value: v1 }, TypeKind::Map { key: k2, value: v2 }) => {
                let key = self.unify(k1, k2)?;
                let value = self.unify(v1, v2)?;
                Ok(self.map_of(key, value))
            }
            (TypeKind::Tuple(t1), TypeKind::Tuple(t2)) if t1.len() == t2.len() => {
                let mut unified = Vec::with_capacity(t1.len());
                for (x, y) in t1.iter().zip(t2.iter()) {
                    unified.push(self.unify(*x, *y)?);
                }
                Ok(self.tuple_of(unified))
            }
            (TypeKind::Function { params: p1, ret: r1 }, TypeKind::Function { params: p2, ret: r2 }) if p1.len() == p2.len() => {
                let mut unified_params = Vec::with_capacity(p1.len());
                for (x, y) in p1.iter().zip(p2.iter()) {
                    unified_params.push(self.unify(*x, *y)?);
                }
                let ret = self.unify(r1, r2)?;
                Ok(self.function_of(unified_params, ret))
            }
            (
                TypeKind::RowRecord { fields: f1, is_open: open1, tail_var: t1, name: n1 },
                TypeKind::RowRecord { fields: f2, is_open: open2, tail_var: t2, name: n2 }
            ) => {
                // Structural row unification
                let mut merged_fields = HashMap::new();
                for (name, ty) in &f1 {
                    merged_fields.insert(name.clone(), *ty);
                }
                for (name, ty) in &f2 {
                    if let Some(existing) = merged_fields.get(name) {
                        let unified = self.unify(*existing, *ty)?;
                        merged_fields.insert(name.clone(), unified);
                    } else if open1 {
                        merged_fields.insert(name.clone(), *ty);
                    } else {
                        return Err(format!("Closed row record missing field '{}'", name));
                    }
                }
                if !open2 {
                    for (name, _) in &f1 {
                        if !f2.iter().any(|(n, _)| n == name) {
                            return Err(format!("Closed row record missing field '{}'", name));
                        }
                    }
                }
                let mut sorted: Vec<(String, TypeId)> = merged_fields.into_iter().collect();
                sorted.sort_by(|x, y| x.0.cmp(&y.0));
                let is_open = open1 && open2;
                let tail = if is_open { t1.or(t2) } else { None };
                let resolved_name = n1.clone().or_else(|| n2.clone());
                Ok(self.row_record_with_name(sorted, is_open, tail, resolved_name))
            }
            (TypeKind::RowRecord { fields, .. }, TypeKind::Map { key, value })
            | (TypeKind::Map { key, value }, TypeKind::RowRecord { fields, .. }) => {
                let unified_key = self.unify(key, TypeId::STR)?;
                let field_tys: Vec<TypeId> = fields.into_iter().map(|(_, t)| t).collect();
                let record_val_ty = self.unify_or_union_all(field_tys);
                let unified_val = self.unify(value, record_val_ty)?;
                Ok(self.map_of(unified_key, unified_val))
            }
            (TypeKind::Typestate { base: b1, state_name: s1 }, TypeKind::Typestate { base: b2, state_name: s2 }) => {
                if s1 != s2 {
                    return Err(format!("Typestate mismatch: '{}' vs '{}'", s1, s2));
                }
                let base = self.unify(b1, b2)?;
                Ok(self.typestate_of(base, s1))
            }
            (TypeKind::Unit { base: b1, unit_name: u1 }, TypeKind::Unit { base: b2, unit_name: u2 }) => {
                if u1 != u2 {
                    return Err(format!("Unit dimension mismatch: '{}' vs '{}'", u1, u2));
                }
                let base = self.unify(b1, b2)?;
                Ok(self.unit_of(base, u1))
            }
            (TypeKind::Adt { name: n1, .. }, TypeKind::Adt { name: n2, .. }) => {
                if n1 == n2 {
                    Ok(a)
                } else {
                    Err(format!("ADT type mismatch: '{}' vs '{}'", n1, n2))
                }
            }
            (TypeKind::Union(types_a), TypeKind::Union(types_b)) => {
                let all_a_in_b = types_a.iter().all(|ta| types_b.contains(ta));
                let all_b_in_a = types_b.iter().all(|tb| types_a.contains(tb));
                if all_a_in_b && all_b_in_a {
                    return Ok(a);
                }
                let mut can_sub = true;
                for &ta in &types_a {
                    let mut found = false;
                    for &tb in &types_b {
                        let prev_subst = self.subst.clone();
                        if self.unify(ta, tb).is_ok() {
                            found = true;
                            break;
                        }
                        self.subst = prev_subst;
                    }
                    if !found {
                        can_sub = false;
                        break;
                    }
                }
                if can_sub {
                    Ok(b)
                } else {
                    Err(format!("Cannot unify union '{}' with '{}'", self.display(a), self.display(b)))
                }
            }
            (TypeKind::Union(types), _) => {
                for &u in &types {
                    self.unify(u, b)?;
                }
                Ok(b)
            }
            (_, TypeKind::Union(types)) => {
                for &u in &types {
                    let prev_subst = self.subst.clone();
                    if self.unify(a, u).is_ok() {
                        return Ok(b);
                    }
                    self.subst = prev_subst;
                }
                Err(format!("Type '{}' is not a member of union '{}'", self.display(a), self.display(b)))
            }
            _ => Err(format!("Cannot unify type {:?} with {:?}", self.get(a), self.get(b))),
        }
    }

    /// Performs structural row monomorphization (Milestone 16.3 / 33.1):
    /// closes an open row record against required concrete fields.
    pub fn monomorphize_row(
        &mut self,
        row_type: TypeId,
        required_fields: &[(String, TypeId)],
    ) -> Result<TypeId, String> {
        let row_type = self.prune(row_type);
        match self.get(row_type).clone() {
            TypeKind::RowRecord { fields, name, .. } => {
                let mut map: HashMap<String, TypeId> = fields.into_iter().collect();
                for (k, v) in required_fields {
                    if let Some(existing) = map.get(k) {
                        let unified = self.unify(*existing, *v)?;
                        map.insert(k.clone(), unified);
                    } else {
                        map.insert(k.clone(), *v);
                    }
                }
                let mut res: Vec<(String, TypeId)> = map.into_iter().collect();
                res.sort_by(|a, b| a.0.cmp(&b.0));
                Ok(self.row_record_with_name(res, false, None, name))
            }
            _ => Err("Target is not a row record".to_string()),
        }
    }

    pub fn display(&self, id: TypeId) -> String {
        let id = self.prune(id);
        if !id.is_primitive() {
            if let Some(name) = self.nominal_names.get(&id) {
                return name.clone();
            }
        }
        match self.get(id) {
            TypeKind::Void => "void".to_string(),
            TypeKind::Never => "never".to_string(),
            TypeKind::Any => "any".to_string(),
            TypeKind::Null => "null".to_string(),
            TypeKind::Primitive(name) => name.clone(),
            TypeKind::Collection { kind, element } => {
                let k_str = match kind {
                    CollectionKind::List => "List",
                    CollectionKind::Set => "Set",
                    CollectionKind::Future => "Future",
                };
                format!("{}[{}]", k_str, self.display(*element))
            }
            TypeKind::Map { key, value } => {
                format!("Map[{}, {}]", self.display(*key), self.display(*value))
            }
            TypeKind::Tuple(elems) => {
                let inner: Vec<String> = elems.iter().map(|e| self.display(*e)).collect();
                format!("({})", inner.join(", "))
            }
            TypeKind::Option(elem) => format!("{}?", self.display(*elem)),
            TypeKind::Result { ok, err } => {
                format!("Result[{}, {}]", self.display(*ok), self.display(*err))
            }
            TypeKind::Function { params, ret } => {
                let p_str: Vec<String> = params.iter().map(|p| self.display(*p)).collect();
                format!("({}) -> {}", p_str.join(", "), self.display(*ret))
            }
            TypeKind::Matrix { rows, cols } => format!("matrix{}x{}", rows, cols),
            TypeKind::Unit { base, unit_name } => format!("{}<{}>", self.display(*base), unit_name),
            TypeKind::Union(types) => {
                let t_str: Vec<String> = types.iter().map(|t| self.display(*t)).collect();
                t_str.join(" | ")
            }
            TypeKind::TypeVar { id, .. } => format!("'T{}", id),
            TypeKind::RowRecord { fields, is_open, name, .. } => {
                if let Some(n) = name {
                    return n.clone();
                }
                let f_str: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.display(*v)))
                    .collect();
                if *is_open {
                    format!("{{ {}, .. }}", f_str.join(", "))
                } else {
                    format!("{{ {} }}", f_str.join(", "))
                }
            }
            TypeKind::Typestate { base, state_name } => {
                format!("{}@{}", self.display(*base), state_name)
            }
            TypeKind::Adt { name, .. } => name.clone(),
        }
    }

    pub fn display_structural(&self, id: TypeId) -> String {
        let id = self.prune(id);
        match self.get(id) {
            TypeKind::RowRecord { fields, is_open, .. } => {
                let f_str: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.display(*v)))
                    .collect();
                if *is_open {
                    format!("{{ {}, .. }}", f_str.join(", "))
                } else {
                    format!("{{ {} }}", f_str.join(", "))
                }
            }
            _ => self.display(id),
        }
    }
}
