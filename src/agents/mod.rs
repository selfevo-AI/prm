//! Digital-world agent gateway. External agents push resource and relation
//! information; the router hands each import to the offline rules or to a local
//! model executor that applies the self ontology as core principles; every
//! result becomes a pending proposal for the owner.
pub mod credentials;
pub mod executor;
pub mod gateway;
pub mod registry;
pub mod router;
