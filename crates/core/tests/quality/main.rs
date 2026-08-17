//! The safety gate end to end: what a plan says about the content it would
//! write, what refuses to install, and what an override does and does not
//! buy.
#![cfg(unix)]

mod decisions;
mod decisions_refuse;
mod fixture;
mod gate;
mod kinds;
mod overrides;
mod reading;
mod review_hash;
mod rules;
mod scoring;
