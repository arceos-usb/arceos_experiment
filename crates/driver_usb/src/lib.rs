//! Common traits and types for graphics display device drivers.

#![no_std]
#![feature(allocator_api)]
#![feature(strict_provenance)]
#![allow(warnings)]
#![feature(auto_traits)]

use core::alloc::Allocator;

extern crate alloc;
pub(crate) mod addr;
pub(crate) mod device_types;
pub(crate) mod dma;
pub mod err;
pub mod host;

#[cfg(feature = "arceos")]
pub mod ax;

pub trait OsDep: Clone + Send + Sync + Sized {
    const PAGE_SIZE: usize;
    type DMA: Allocator + Send + Sync;
    fn dma_alloc(&self) -> Self::DMA;
    fn force_sync_cache();
}
