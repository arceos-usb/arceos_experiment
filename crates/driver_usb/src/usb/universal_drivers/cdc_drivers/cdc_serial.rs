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
use xhci::ring::trb::transfer::Direction;
use crate::{
    usb::trasnfer::control::{bRequest, bmRequestType, ControlTransfer, DataTransferType, Recipient},
    usb::descriptors::topological_desc::{TopologicalUSBDescriptorFunction,
        TopologicalUSBDescriptorEndpoint},
    };

pub struct CdcSerialDriver<O>
where
    O: PlatformAbstractions,
{
    device_slot_id: usize,
    endpoints: Vec<Endpoint>,
    config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    interface_value: usize,
    config_value: usize,
}

impl<'a, O> CdcSerialDriver<O>
where
    O: PlatformAbstractions + 'static,
{
    pub fn new_and_init(
        device_slot_id: usize,
        endpoints: Vec<Endpoint>,
        config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
        interface_value: usize,
        config_value: usize,
    ) -> Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>> {
        trace!("CdcSerialDriver initializing");
        trace!("endpoints: {:?}", endpoints);
        Arc::new(SpinNoIrq::new(
        Self {
                device_slot_id,
                endpoints,
                config,
                interface_value,
                config_value,
                }
            )
        )      
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
        trace!("CdcSerialDriver preparing for drive");
        let mut todo_list = Vec::new();
        //设置配置描述符
        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::Out,
                    DataTransferType::Standard,
                    Recipient::Device,
                ),
                request: bRequest::SetConfiguration,
                index: self.interface_value as u16,
                value: self.config_value as u16,
                data: None,
                response: true,
            }),
        ));
        Some(todo_list)
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
            let vendor = device.data.vendor;
            let product_id = device.data.product_id;
            trace!("vendor: {:x}, product_id: {:x}", vendor, product_id);
            if device.data.class == StandardUSBDeviceClassCode::VendorSpecific as u8
                && vendor == 0x1a86
                && product_id == 0x7523
            {
                let driver = CdcSerialDriver::new_and_init(
                    independent_dev.slotid,
                    {
                        device
                            .child
                            .iter()
                            .find(|c| {
                                c.data.config_val() == independent_dev.configuration_val as u8
                            })
                            .expect("configuration not found")
                            .child
                            .iter()
                            .filter_map(|func| match func {
                                TopologicalUSBDescriptorFunction::InterfaceAssociation(_) => {
                                    panic!("a super complex device, help meeeeeeeee!hhhhh ");
                                }
                                TopologicalUSBDescriptorFunction::Interface(interface) => Some(
                                    interface
                                        .iter()
                                        .find(|(interface, alternatives, endpoints)| {
                                            interface.interface_number
                                                == independent_dev.interface_val as u8
                                                && interface.alternate_setting
                                                    == independent_dev
                                                        .current_alternative_interface_value
                                                        as u8
                                        })
                                        .expect("invalid interface value or alternative value")
                                        .2
                                        .clone(),
                                ),
                            })
                            .take(1)
                            .flat_map(|a| a)
                            .filter_map(|e| {
                                if let TopologicalUSBDescriptorEndpoint::Standard(ep) = e {
                                    Some(ep)
                                } else {
                                    None
                                }
                            })
                            .collect()
                    },
                    config.clone(),
                    independent_dev.interface_val,
                    independent_dev.configuration_val,);
                trace!("activating CDC serial driver");
                trace!("Vendor id:0x1a86 QinHeng Electronics");
                trace!("Product id:0x7523 CH340 serial converter");
                trace!("CH340 configuration value:{}", independent_dev.configuration_val);
                trace!("CH340 interface value:{}", independent_dev.interface_val);
                Some(vec![driver])
            } else {
                None
            }
        } else {
            None
        }
    }
    fn preload_module(&self) {
        trace!("preloading Hid mouse driver!")
    }
}
