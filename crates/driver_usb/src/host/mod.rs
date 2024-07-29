use alloc::{boxed::Box, collections::binary_heap::Iter, sync::Arc, vec::Vec};
use data_structures::host_controllers::{xhci::XHCI, Controller, ControllerArc};
use log::trace;
use spinlock::SpinNoIrq;
use xhci::ring::trb::event;

use crate::{
    abstractions::PlatformAbstractions,
    glue::driver_independent_device_instance::DriverIndependentDeviceInstance,
    usb::{self, operation::Configuration, trasnfer::control::ControlTransfer, urb::URB},
    USBSystemConfig,
};

pub mod data_structures;

impl<O> USBSystemConfig<O>
where
    O: PlatformAbstractions,
{
    pub fn new(mmio_base_addr: usize, irq_num: u32, irq_priority: u32, os_dep: O) -> Self {
        let base_addr = O::VirtAddr::from(mmio_base_addr);
        Self {
            base_addr,
            irq_num,
            irq_priority,
            os: os_dep,
        }
    }
}

#[derive(Clone)]
pub struct USBHostSystem<O>
where
    O: PlatformAbstractions,
{
    config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    controller: ControllerArc<O>,
}

impl<O> USBHostSystem<O>
where
    O: PlatformAbstractions + 'static,
{
    pub fn new(config: Arc<SpinNoIrq<USBSystemConfig<O>>>) -> crate::err::Result<Self> {
        let controller = Arc::new(SpinNoIrq::new({
            let xhciregisters: Box<(dyn Controller<O> + 'static)> = {
                if cfg!(feature = "xhci") {
                    Box::new(XHCI::new(config.clone()))
                } else {
                    panic!("no host controller defined")
                }
            };
            xhciregisters
        }));
        Ok(Self { config, controller })
    }

    pub fn init(&self) {
        self.controller.lock().init()
    }

    pub fn probe<F>(&self, consumer: F)
    where
        F: FnMut(DriverIndependentDeviceInstance<O>),
    {
        let mut probe = self.controller.lock().probe();
        probe
            .iter()
            .map(|slot_id| {
                DriverIndependentDeviceInstance::new(slot_id.clone(), self.controller.clone())
            })
            .for_each(consumer);
    }

    pub fn control_transfer(
        &mut self,
        dev_slot_id: usize,
        urb_req: ControlTransfer,
    ) -> crate::err::Result {
        self.controller
            .lock()
            .control_transfer(dev_slot_id, urb_req)
    }

    pub fn configure_device(
        &mut self,
        dev_slot_id: usize,
        urb_req: Configuration,
    ) -> crate::err::Result {
        self.controller
            .lock()
            .configure_device(dev_slot_id, urb_req)
    }

    pub fn urb_request(&mut self, request: URB<O>) -> crate::err::Result {
        // trace!("request {:#?}", request);
        match request.operation {
            usb::urb::RequestedOperation::Control(control) => {
                self.control_transfer(request.device_slot_id, control)
            }
            usb::urb::RequestedOperation::Bulk => todo!(),
            usb::urb::RequestedOperation::Interrupt(interrupt_transfer) => self
                .controller
                .lock()
                .interrupt_transfer(request.device_slot_id, interrupt_transfer),
            usb::urb::RequestedOperation::Isoch => todo!(),
            usb::urb::RequestedOperation::ConfigureDevice(configure) => self
                .controller
                .lock()
                .configure_device(request.device_slot_id, configure),
        }
    }
}
