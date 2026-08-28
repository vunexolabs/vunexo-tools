//! Use-case orchestration. Depends on `crate::domain` and on ports it
//! defines itself; never reaches directly into `crate::infrastructure`
//! (dependency inversion — infrastructure implements the ports defined here).
//! Real use cases (create invoice, record payment, ...) land in later rounds.
