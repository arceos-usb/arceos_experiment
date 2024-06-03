pub mod usb;
pub mod xhci;
pub mod device;
use core::alloc::Allocator;
use alloc::{boxed::Box, sync::Arc};
use spinlock::SpinNoIrq;

use crate::{addr::VirtAddr, err::*, OsDep};

#[derive(Clone)]
pub struct USBHostConfig<O>
where O: OsDep
{
    pub(crate) base_addr: VirtAddr,
    pub(crate) irq_num: u32,
    pub(crate) irq_priority: u32,
    pub(crate) os: O
}

impl <O> USBHostConfig <O>
where O: OsDep
{
    pub fn new(mmio_base_addr: usize, irq_num: u32, irq_priority: u32, os_dep:  O)->Self{
        let base_addr = VirtAddr::from(mmio_base_addr);
        Self { base_addr, irq_num, irq_priority, os: os_dep }
    }
}

pub trait Controller<O>: Send + Sync
where O: OsDep
{
    fn new(config: USBHostConfig<O>) -> Result<Self> where Self: Sized;
    fn poll(&self)->Result;
}

#[derive(Clone)]
pub struct USBHost<O>
where O: OsDep
{
    pub(crate) config: USBHostConfig<O>,
    pub(crate) controller: Arc<dyn Controller<O>>,
}

impl <O> USBHost<O>
where O: OsDep
{
    pub fn new<C: Controller<O> + 'static>(config: USBHostConfig<O>) -> Result<Self> {
        let controller : Arc<dyn Controller<O>>= Arc::new(C::new(config.clone())?);
        // let controller = Arc::new( SpinNoIrq::new(controller));
        Ok(Self { config, controller })
    }

    pub fn poll(&self)->Result{
        self.controller.poll()
    }
}