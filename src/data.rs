use bevy::prelude::*;
use std::{
    cmp::Ordering,
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};

pub(crate) fn plugin(app: &mut App) {
    app.insert_resource(HtnSettings::default());
}

type UniqueNameStorage = &'static str;
pub static UNIQUE_NAME_REGISTRY: LazyLock<Mutex<HashMap<String, Arc<UniqueNameStorage>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, PartialEq, PartialOrd, Debug, Eq, Hash)]
pub struct UniqueName(Arc<UniqueNameStorage>);

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Variant {
    Bool(bool),
    String(UniqueName),
    Number(f32),
}

#[derive(Default, Clone, Debug, PartialEq, Resource)]
/// For an HTN, a context is simply a collection of known 'predicate's.
pub struct WorldState {
    entries: HashMap<UniqueName, Variant>,
}

#[derive(Default, Clone, Debug, PartialEq)]
pub enum Predicate {
    #[default]
    HasEntry,
    Equals(Variant),
    /// uses partialeq (or totaleq for Number) to compare. Returns true if the comparison Ordering matches the stored Ordering
    Order(Ordering, Variant),
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct Requirements {
    entries: HashMap<UniqueName, Predicate>,
}

#[derive(Default, Clone, Debug, PartialEq, Resource)]
pub struct HtnSettings {
    pub frame_processing_limit: Option<Duration>,
    pub node_branch_limit: Option<u32>,
    pub disable_priority_sort: Option<bool>,
}

impl UniqueName {
    pub fn new(string: &'static str) -> Self {
        let mut lock = UNIQUE_NAME_REGISTRY
            .lock()
            .expect("Propagating mutex thread panic");

        Self(
            lock.entry(string.into())
                .or_insert(Arc::new(string))
                .clone(),
        )
    }
}

impl WorldState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, key: impl Into<UniqueName>, value: impl Into<Variant>) -> &mut Self {
        self.insert(key, value);
        self
    }

    pub fn build(&mut self) -> Self {
        self.clone()
    }

    pub fn insert(
        &mut self,
        key: impl Into<UniqueName>,
        value: impl Into<Variant>,
    ) -> Option<Variant> {
        self.entries.insert(key.into(), value.into())
    }

    pub fn erase(&mut self, key: impl Into<UniqueName>) {
        self.entries.remove(&key.into());
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// ensure that the other world's set of truths is a subset of this World's truths.
    /// Early exit if a value in other is not present in this world or if the values between worlds do not match
    pub fn validate(&self, other: &WorldState) -> bool {
        for (name, truth) in &other.entries {
            let Some(val) = self.entries.get(name) else {
                return false;
            };
            if *val != *truth {
                return false;
            }
        }
        return true;
    }

    pub fn get(&self, s: impl Into<UniqueName>) -> Option<Variant> {
        let Some(value) = self.entries.get(&s.into()) else {
            return None;
        };
        return Some(value.clone());
    }

    pub fn append(&mut self, other: &WorldState) {
        for (name, truth) in &other.entries {
            self.entries.insert(name.clone(), truth.clone());
        }
    }

    pub fn concat(&self, other: &WorldState) -> Self {
        let mut n_world = self.clone();
        n_world.append(other);
        n_world
    }
}

impl Requirements {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn validate(&self, world: &WorldState) -> bool {
        for (key, value) in self.entries.iter() {
            let Some(var) = world.get(key.clone()) else {
                return false;
            };
            if !value.validate(var) {
                return false;
            }
        }
        true
    }

    pub fn consume(&self, world: &WorldState) -> WorldState {
        let mut reduced_world = world.clone();
        for (key, value) in self.entries.iter() {
            let Some(var) = world.get(key.clone()) else {
                continue;
            };
            if value.validate(var) {
                reduced_world.erase(key.clone()); // purge entries that meet requirements
            }
        }
        reduced_world
    }

    pub fn unmet_requirements(&self, world: &WorldState) -> Requirements {
        let mut reduced_req = self.clone();
        for (key, value) in self.entries.iter() {
            let Some(var) = world.get(key.clone()) else {
                continue;
            };
            if value.validate(var) {
                reduced_req.entries.remove(key); // purge entries that meet requirements
            }
        }
        reduced_req
    }

    pub fn req(
        &mut self,
        key: impl Into<UniqueName>,
        predicate: impl Into<Predicate>,
    ) -> &mut Self {
        self.entries.insert(key.into(), predicate.into());
        self
    }

    pub fn req_equals(
        &mut self,
        key: impl Into<UniqueName>,
        variant: impl Into<Variant>,
    ) -> &mut Self {
        self.req(key, Predicate::Equals(variant.into()));
        self
    }

    pub fn req_greater(
        &mut self,
        key: impl Into<UniqueName>,
        variant: impl Into<Variant>,
    ) -> &mut Self {
        self.req(key, Predicate::Order(Ordering::Greater, variant.into()));
        self
    }

    pub fn req_less(
        &mut self,
        key: impl Into<UniqueName>,
        variant: impl Into<Variant>,
    ) -> &mut Self {
        self.req(key, Predicate::Order(Ordering::Less, variant.into()));
        self
    }

    pub fn req_has(&mut self, key: impl Into<UniqueName>) -> &mut Self {
        self.req(key.into(), Predicate::HasEntry);
        self
    }

    pub fn append(&mut self, req: &Requirements) {
        self.entries = self
            .entries
            .iter()
            .chain(req.entries.iter())
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
    }

    pub fn build(&mut self) -> Self {
        self.clone()
    }
}

impl<I, S> From<I> for WorldState
where
    I: Iterator<Item = (S, Variant)>,
    S: Into<UniqueName>,
{
    fn from(value: I) -> Self {
        let mut map = HashMap::new();

        for (name, truth) in value {
            let un: UniqueName = name.into();
            if let Some(_) = map.insert(un.clone(), truth) {
                warn!("Duplicate entries for key: {:?}", un);
            }
        }
        Self { entries: map }
    }
}

impl Predicate {
    pub fn validate(&self, variant: Variant) -> bool {
        match self {
            Predicate::HasEntry => true,
            Predicate::Equals(var) => variant == *var,
            Predicate::Order(ord, var) => {
                if let Variant::Number(num) = var {
                    if let Variant::Number(num2) = variant {
                        return num2.total_cmp(num) == *ord;
                    }
                }
                if let Some(pord) = variant.partial_cmp(var) {
                    return pord == *ord;
                }
                false
            }
        }
    }
}

impl From<Variant> for WorldState {
    fn from(value: Variant) -> Self {
        let mut map = HashMap::new();
        map.insert("value".into(), value);
        Self { entries: map }
    }
}

impl From<bool> for Variant {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<UniqueName> for Variant {
    fn from(value: UniqueName) -> Self {
        Self::String(value)
    }
}

impl Into<Variant> for &'static str {
    fn into(self) -> Variant {
        Variant::String(self.into())
    }
}

impl From<f32> for Variant {
    fn from(value: f32) -> Self {
        Self::Number(value)
    }
}

impl Into<Requirements> for WorldState {
    /// Converts the world state to a Requirements struct with all predicates being `Equals`.
    /// Not terribly customizeable, but that's what you get for taking the easy way you rapscallion!
    fn into(self) -> Requirements {
        let mut req = Requirements::new();
        for (key, var) in self.entries {
            req.req_equals(key, var);
        }
        req
    }
}

impl Into<UniqueName> for &'static str {
    fn into(self) -> UniqueName {
        UniqueName::new(self)
    }
}

impl Default for Variant {
    fn default() -> Self {
        Self::Bool(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truth_equality() {
        let bool_true = Variant::Bool(true);
        let bool_false = Variant::Bool(false);
        let string_empty = Variant::String("".into());
        let string_test = Variant::String("test".into());

        // bools matrix
        assert_eq!(bool_true, Variant::Bool(true));
        assert_eq!(bool_false, Variant::Bool(false));

        assert_ne!(bool_true, bool_false);
        assert_ne!(bool_true, Variant::Bool(false));
        assert_ne!(bool_false, Variant::Bool(true));

        // strings matrix
        assert_eq!(string_empty, Variant::String("".into()));
        assert_eq!(string_test, Variant::String("test".into()));

        assert_ne!(string_empty, string_test);

        // cross comparisons
        assert_ne!(bool_true, string_empty);
        assert_ne!(bool_true, string_test);

        assert_ne!(bool_false, string_empty);
        assert_ne!(bool_false, string_test);

        assert_ne!(string_empty, bool_true);
        assert_ne!(string_empty, bool_false);

        assert_ne!(string_test, bool_true);
        assert_ne!(string_test, bool_false);
    }

    #[test]
    fn test_world_validation() {
        let truths_base: WorldState = vec![
            ("safe", true.into()),
            ("running", false.into()),
            ("happy", true.into()),
        ]
        .into_iter()
        .into();
        let truths_valid: WorldState = vec![("safe", true.into()), ("happy", true.into())]
            .into_iter()
            .into();
        let truths_invalid: WorldState = vec![("running", true.into())].into_iter().into();

        assert!(truths_base.validate(&truths_valid));
        assert!(!truths_base.validate(&truths_invalid));

        assert!(!truths_valid.validate(&truths_base));
        assert!(!truths_valid.validate(&truths_invalid));

        assert!(!truths_invalid.validate(&truths_base));
        assert!(!truths_invalid.validate(&truths_valid));

        let concatenated_set: WorldState = vec![("running", true.into())].into_iter().into();
        let super_set = truths_base.concat(&concatenated_set);
        assert!(!truths_base.validate(&truths_invalid)); // ensure that concating doesn't change base value
        assert!(super_set.validate(&truths_invalid)); // ensure new concatenation is valid for both
        assert!(super_set.validate(&truths_valid)); // ensure new concatenation is valid for both
    }
}
