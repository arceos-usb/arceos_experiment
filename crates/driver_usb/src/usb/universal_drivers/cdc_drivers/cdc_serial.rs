use alloc::sync::Arc;
use alloc::vec;
use crate::{abstractions::PlatformAbstractions, usb::{
    descriptors::{desc_device::StandardUSBDeviceClassCode, desc_endpoint::Endpoint}, //todo:check if this is nessesary
    drivers::driverapi::{USBSystemDriverModule, USBSystemDriverModuleInstance},
}, USBSystemConfig};
use crate::host::data_structures::MightBeInited;
use crate::usb::urb::URB;
use log::trace;
use spinlock::SpinNoIrq;
use alloc::vec::Vec;
use crate::glue::ucb::{CompleteCode, TransferEventCompleteCode, UCB};
use crate::usb::urb::{RequestedOperation};
use crate::{
    glue::driver_independent_device_instance::DriverIndependentDeviceInstance,
};
pub struct CdcSerialDriver<O>
where
    O: PlatformAbstractions,
{
    config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
}

impl<'a, O> CdcSerialDriver<O>
where
    O: PlatformAbstractions + 'static,
{
    pub fn new(
        config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    ) -> Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>> {
        Arc::new(SpinNoIrq::new(Self { config }))
    }
}

impl<'a, O> USBSystemDriverModuleInstance<'a, O> for CdcSerialDriver<O>
where
    O: PlatformAbstractions,
{
    fn gather_urb(&mut self) -> Option<Vec<crate::usb::urb::URB<'a, O>>> {
        todo!()
    }
    fn receive_complete_event(&mut self, ucb: UCB<O>) {
        todo!()
    }
    fn prepare_for_drive(&mut self) -> Option<Vec<URB<'a, O>>> {
        todo!()
    }
}

pub struct CdcSerialDriverModule;

impl<'a, O> USBSystemDriverModule<'a, O> for CdcSerialDriverModule
where
    O: PlatformAbstractions + 'static,
{
    fn should_active(
        &self,
        independent_dev: &DriverIndependentDeviceInstance<O>,
        config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    ) -> Option<Vec<Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>>>> {
        trace!("checking if we should activate CDC serial driver");
        if let MightBeInited::Inited(topologicalUSBDescriptorRoot) = &*independent_dev.descriptors
        {
            let device = &topologicalUSBDescriptorRoot.device.first().unwrap();
            trace!("device class: {:?}", device.data.class);
            return match StandardUSBDeviceClassCode::from(device.data.class) {
                StandardUSBDeviceClassCode::CommunicationsAndCDCControl => {
                    trace!("activating CDC serial driver!");
                    Some(vec![CdcSerialDriver::new(config.clone())])
                }
                _ => None,
            };
        } else {
            None
        }
    }
    fn preload_module(&self) {
        trace!("preloading Hid mouse driver!")
    }
}
