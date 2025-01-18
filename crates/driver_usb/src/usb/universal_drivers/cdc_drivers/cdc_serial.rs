use crate::{
    abstractions::PlatformAbstractions,
    usb::{
        descriptors::{desc_device::StandardUSBDeviceClassCode, desc_endpoint::Endpoint}, //todo:check if this is nessesary
        drivers::driverapi::{USBSystemDriverModule, USBSystemDriverModuleInstance},
    },
};

pub struct CdcSerialDriver<O>
where
    O: PlatformAbstractions,
{
    ops: O,
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
        todo!()
    }
    fn preload_module(&self) {
        trace!("preloading Hid mouse driver!")
    }
}
