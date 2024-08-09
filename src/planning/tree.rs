use std::sync::Arc;

/// Super basic non-cyclic, directional graph (aka a tree)
/// Uses Arc because I want/need it to be Send/Sync
pub struct Node<T> {
    pub value: T,
    pub parent: Option<Arc<Node<T>>>, // god I wish there was a better ref-counted smart pointer that is Send/Sync
}
