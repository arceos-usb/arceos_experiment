use core::ptr;

//
use alloc::vec;
use alloc::vec::Vec;
use log::{debug, error, trace, warn};
use num_traits::FromPrimitive;

use crate::{
    abstractions::{dma::DMA, PlatformAbstractions},
    usb::descriptors::{
        desc_hid::Hid,
        desc_uvc::uvc_interfaces::{UVCControlInterface, UVCInterface, UVCStreamingInterface},
        USBDescriptor, USBStandardDescriptorTypes,
    },
};

use super::{
    desc_device::StandardUSBDeviceClassCode,
    desc_interface::{Interface, InterfaceAssociation},
    desc_uvc::{
        uvc_interfaces::{
            UVCInterfaceSubclass, UVCStandardVideoInterfaceClass,
            UVCStandardVideoInterfaceProtocols,
        },
        UVCDescriptorTypes,
    },
    topological_desc::{
        TopologicalUSBDescriptorConfiguration, TopologicalUSBDescriptorDevice,
        TopologicalUSBDescriptorEndpoint, TopologicalUSBDescriptorFunction,
        TopologicalUSBDescriptorRoot,
    },
};

pub(crate) struct RawDescriptorParser<O: PlatformAbstractions> {
    device: DMA<[u8], O::DMA>,
    configs: Vec<(DMA<[u8], O::DMA>, usize)>,
    state: ParserStateMachine,
    result: Option<TopologicalUSBDescriptorDevice>,
    others: Vec<USBDescriptor>,
    metadata: ParserMetaData,
    current: usize,
    current_len: usize,
}

#[derive(Debug)]
pub(crate) enum Error {
    UnrecognizedType(u8),
    ParseOrderError,
    EndOfDescriptors,
    NotReadyToParse,
    StateSwitch,
}

#[derive(PartialEq, Debug)]
enum ParserStateMachine {
    Device,
    NotReady,
    Config(usize),
    Inetrface(usize, u8),
    END,
}

#[derive(Clone, Debug)]
pub enum ParserMetaData {
    USBToSerial,
    UVC(u8),
    HID,
    Unknown(ParserMetaDataUnknownSituation),
    NotDetermined,
}

#[derive(Clone, Debug)]
pub enum ParserMetaDataUnknownSituation {
    Unknown, //treat as standard usb device
    ReferIAC,
    ReferInterface,
}

impl ParserMetaData {
    //refer https://www.usb.org/defined-class-codes
    pub fn determine(class: u8, subclass: u8, protocol: u8) -> Self {
        trace!("determine metadata from class:{},subclass:{},protocol:{}", class, subclass, protocol);
        match (class.into(), subclass, protocol) {
            (StandardUSBDeviceClassCode::Miscellaneous, 0x02, 0x01) => {
                return Self::Unknown(ParserMetaDataUnknownSituation::ReferIAC)
            }
            (StandardUSBDeviceClassCode::HID, _, _) => return Self::HID,
            (StandardUSBDeviceClassCode::VendorSpecific,0,0)=> {
                return Self::USBToSerial
            }
            (StandardUSBDeviceClassCode::ReferInterfaceDescriptor, _, _) => {
                return Self::Unknown(ParserMetaDataUnknownSituation::ReferInterface)
            }
            _ => {}
        }

        match (class.into(), subclass.into(), protocol.into()) {
            (
                UVCStandardVideoInterfaceClass::CC_Video,
                UVCInterfaceSubclass::VIDEO_INTERFACE_COLLECTION,
                UVCStandardVideoInterfaceProtocols::PC_PROTOCOL_UNDEFINED,
            ) => return Self::UVC(0u8),
            _ => {}
        }

        Self::Unknown(ParserMetaDataUnknownSituation::Unknown)
    }
}

impl<O> RawDescriptorParser<O>
where
    O: PlatformAbstractions,
{
    pub fn new(raw_device: DMA<[u8], O::DMA>) -> Self {
        let len = raw_device.len();
        Self {
            device: raw_device,
            configs: Vec::new(),
            state: ParserStateMachine::Device,
            current: 0,
            current_len: len,
            result: None,
            others: Vec::new(),
            metadata: ParserMetaData::NotDetermined,
        }
    }

    pub fn num_of_configs(&self) -> usize {
        if self.state != ParserStateMachine::Device {
            self.result
                .as_ref()
                .map(|r| r.data.num_configurations.clone() as _)
                .unwrap()
        } else {
            panic!("do not call this method before device has been deserialized!");
        }
    }

    pub fn append_config(&mut self, raw_config: DMA<[u8], O::DMA>) -> &mut Self {
        let len = raw_config.len();
        self.configs.push((raw_config, len));
        self
    }

    pub fn summarize(mut self) -> TopologicalUSBDescriptorRoot {
        while self.single_state_cycle() {}
        TopologicalUSBDescriptorRoot {
            device: vec![self.result.unwrap()],
            others: self.others,
            metadata: self.metadata,
        }
    }

    //return false if reach end, otherwise true
    pub fn single_state_cycle(&mut self) -> bool {
        match &self.state {
            ParserStateMachine::Device => {
                trace!("parse single device desc!");
                self.result = self.parse_single_device_descriptor().ok();
                self.state = ParserStateMachine::NotReady;
                trace!("state change:{:?}", self.state);
                self.current = 0; 
                self.current_len = 0;
                true
            }
            ParserStateMachine::Config(index) => {
                let num_of_configs = self.num_of_configs();
                trace!("parse config desc!,num of configs:{}", num_of_configs);
                let current_index = *index;
                trace!("current index:{}", current_index);
                if current_index >= num_of_configs {
                    self.state = ParserStateMachine::END;
                    trace!("state change:{:?}", self.state);
                    return false;
                }
                let topological_usbdescriptor_configuration = self.parse_current_config().unwrap();
                self.result
                    .as_mut()
                    .unwrap()
                    .child
                    .push(topological_usbdescriptor_configuration);
                self.state = ParserStateMachine::Config(current_index + 1);
                trace!("state change:{:?}", self.state);
                true
            }
        
            ParserStateMachine::END => panic!("should not call anymore while reaching end"),
            ParserStateMachine::NotReady => {
                if let Some(res) = &self.result
                    && self.configs.len() >= res.data.num_configurations as _
                {
                    self.state = ParserStateMachine::Config(0);
                    trace!("state change:{:?}", self.state);
                    self.current_len = self.configs[0].1;
                    true
                } else {
                    false
                }
            }
            _ => true,
        }
    }

    fn cut_raw_descriptor(&mut self) -> Result<Vec<u8>, Error> {
        match &self.state {
            ParserStateMachine::Device => {
                let len: usize = self.device[self.current].into();
                let v = self.device[self.current..(self.current + len)].to_vec();
                self.current += len;
                Ok(v)
            }
            ParserStateMachine::NotReady => Err(Error::NotReadyToParse),
            ParserStateMachine::Config(cfg_index) | ParserStateMachine::Inetrface(cfg_index, _) => {
                let len: usize = (self.configs[*cfg_index].0)[self.current].into();
                let v = (self.configs[*cfg_index].0)[self.current..(self.current + len)].to_vec();
                self.current += len;
                Ok(v)
            }
            ParserStateMachine::END => Err(Error::EndOfDescriptors),
        }
    }

    fn parse_single_device_descriptor(&mut self) -> Result<TopologicalUSBDescriptorDevice, Error> {
        trace!("parse single device desc!");
        if let USBDescriptor::Device(dev) = self.parse_any_descriptor()? {
            {
                trace!("parsed device.len:{}", dev.len);
                trace!("parsed device.descriptor_type:{}", dev.descriptor_type);
                let cd_usb = dev.cd_usb;
                trace!("parsed device.cd_usb:{:x}", cd_usb);
                trace!("parsed device.class:{}", dev.class);
                trace!("parsed device.subclass:{}", dev.subclass);
                trace!("parsed device.protocol:{}", dev.protocol);
                trace!("parsed device.max_packet_size0:{}", dev.max_packet_size0);
                let vendor = dev.vendor;
                trace!("parsed device.vendor:{:x}", vendor);
                let product_id = dev.product_id;
                trace!("parsed device.product_id:{:x}", product_id);
                let devicee = dev.device;
                trace!("parsed device.device:{}", devicee);
                trace!("parsed device.manufacture:{}", dev.manufacture);
                trace!("parsed device.product:{}", dev.product);
                trace!("parsed device.serial_number:{}", dev.serial_number);
                trace!("parsed device.num_configurations:{}", dev.num_configurations);
                match self.metadata {
                    ParserMetaData::NotDetermined => {
                        trace!("ParserMetaData::NotDetermined");
                        self.metadata =
                            ParserMetaData::determine(dev.class, dev.subclass, dev.protocol)
                    }
                    _ => {}
                }
            };
            Ok(TopologicalUSBDescriptorDevice {
                data: dev,
                child: Vec::new(),
            })
        } else {
            Err(Error::ParseOrderError)
        }
    }

    fn parse_current_config(&mut self) -> Result<TopologicalUSBDescriptorConfiguration, Error> {
        trace!("parse config desc!");
        let raw = self.cut_raw_descriptor()?;

        let mut cfg =
            USBDescriptor::from_slice(&raw, self.metadata.clone()).and_then(|converted| {
                if let USBDescriptor::Configuration(cfg) = converted {
                    trace!("get ch340 config desc!");
                    trace!("parsed config.len:{}", cfg.length());
                    trace!("parsed config.ty:{}", cfg.ty());
                    trace!("parsed config.total_length:{}", cfg.total_length());
                    trace!("parsed config.num_interfaces:{}", cfg.num_interfaces());
                    trace!("parsed config.config_val:{}", cfg.config_val());
                    trace!("parsed config.config_string:{}", cfg.config_string());
                    trace!("parsed config.attributes:{}", cfg.attributes());
                    trace!("parsed config.max_power:{}", cfg.max_power());
                    Ok(TopologicalUSBDescriptorConfiguration {
                        data: cfg,
                        child: Vec::new(),
                    })
                } else {
                    Err(Error::ParseOrderError)
                }
            })?;

        trace!("max num of interface num:{}", cfg.data.num_interfaces());

        loop {
            match self.parse_function() {
                Ok(func) => {
                    cfg.child.push(func);
                }
                Err(Error::EndOfDescriptors) => {
                    break;
                }
                Err(Error::StateSwitch) => {
                    continue;
                }
                Err(other) => return Err(other),
            }
        }

        Ok(cfg)
    }

    fn parse_function(&mut self) -> Result<TopologicalUSBDescriptorFunction, Error> {
        trace!("parse function desc!");

        if let Some(desc_type) = self.peek_std_desc_type() {
            match desc_type {
                USBStandardDescriptorTypes::Interface => {
                    trace!(
                        "parse single interface desc! current state:{:?}",
                        self.state
                    );
                    // let collections = TopologicalUSBDescriptorFunction::Interface(vec![]);
                    let mut interfaces = Vec::new();

                    loop {
                        trace!("loop! state:{:?}", self.state);
                        match &self.state {
                            ParserStateMachine::Config(cfg_id) => {
                                self.state = ParserStateMachine::Inetrface(
                                    (*cfg_id),
                                    self.peek_interface().unwrap().interface_number,
                                );
                                trace!("state change:{:?}", self.state);
                            }
                            ParserStateMachine::Inetrface(cfg_index, current_interface_id) => {
                                trace!("state interface!");
                                match &self.peek_interface() {
                                    Some(next)
                                        if (next.interface_number) == *current_interface_id =>
                                    {
                                        trace!("equal!");
                                        trace!("current:{:?}", current_interface_id);
                                        let interface = self.parse_interface().unwrap();
                                        trace!("got interface {:?}", interface);
                                        let additional = self.parse_other_descriptors_by_metadata();
                                        trace!("got additional data {:?}", additional);
                                        let endpoints = self.parse_endpoints();
                                        trace!("got endpoints {:?}", endpoints);
                                        interfaces.push((interface, additional, endpoints))
                                    }
                                    Some(next)
                                        if (next.interface_number) != *current_interface_id =>
                                    {
                                        trace!("not equal!");
                                        self.state = ParserStateMachine::Inetrface(
                                            *cfg_index,
                                            next.interface_number,
                                        );
                                        trace!("state change:{:?}", self.state);
                                        break;
                                    }
                                    None => break,
                                    other => panic!("deserialize error! {:?}", other),
                                };
                            }
                            _ => panic!("impossible situation!"),
                        }
                    }

                    Ok(TopologicalUSBDescriptorFunction::Interface(interfaces))
                }
                USBStandardDescriptorTypes::InterfaceAssociation => {
                    trace!("parse InterfaceAssociation desc!");
                    let interface_association = self.parse_interface_association().unwrap();
                    // match &self.state {
                    //     ParserStateMachine::Config(cfg_id) => {
                    //         self.state = ParserStateMachine::Inetrface(cfg_id.clone(), 0);
                    //         trace!("state change:{:?}", self.state);
                    //     }
                    //     other => panic!("error on switching state! {:?}", other),
                    // }
                    let mut interfaces = Vec::new();
                    for i in 0..interface_association.interface_count {
                        trace!("parsing {i}th interface!");
                        //agreement:there is always some interfaces that match the cound behind association
                        interfaces.push(self.parse_function()?);
                    }
                    Ok(TopologicalUSBDescriptorFunction::InterfaceAssociation((
                        interface_association,
                        interfaces,
                    )))
                }
                anyother => {
                    trace!("unrecognize type!");
                    Err(Error::UnrecognizedType(anyother as u8))
                }
            }
        } else {
            Err(Error::EndOfDescriptors)
        }
    }

    fn parse_any_descriptor(&mut self) -> Result<USBDescriptor, Error> {
        trace!(
            "parse any desc at current{}! type:{:?}",
            self.current,
            self.peek_std_desc_type()
        );
        let raw = self.cut_raw_descriptor()?;
        USBDescriptor::from_slice(&raw, self.metadata.clone())
    }

    fn parse_interface_association(&mut self) -> Result<InterfaceAssociation, Error> {
        match self.parse_any_descriptor()? {
            USBDescriptor::InterfaceAssociation(interface_association) => {
                if let ParserMetaData::Unknown(ParserMetaDataUnknownSituation::ReferIAC) =
                    self.metadata
                {
                    self.metadata = ParserMetaData::determine(
                        interface_association.function_class,
                        interface_association.function_subclass,
                        interface_association.function_protocol,
                    )
                }

                Ok(interface_association)
            }
            _ => Err(Error::ParseOrderError),
        }
    }

    fn peek_std_desc_type(&self) -> Option<USBStandardDescriptorTypes> {
        match self.state {
            ParserStateMachine::Device => {
                let peeked =
                    USBStandardDescriptorTypes::from_u8(self.device[self.current + 1] as u8);
                trace!("peeked type:{:?}", peeked);
                peeked
            }
            ParserStateMachine::Config(index) | ParserStateMachine::Inetrface(index, _) => {
                let peeked = USBStandardDescriptorTypes::from_u8(
                    self.configs[index].0[self.current + 1] as u8,
                );
                trace!("peeked std type:{:?}", peeked);
                peeked
            }
            _ => panic!("impossible!"),
        }
    }

    //while call this methods, parser state machine always at "config" state
    fn peek_uvc_desc_type(&mut self) -> Option<UVCDescriptorTypes> {
        trace!("peek uvc type!");
        match self.state {
            ParserStateMachine::Config(index) | ParserStateMachine::Inetrface(index, _) => {
                UVCDescriptorTypes::from_u8(self.configs[index].0[self.current + 1])
            }
            _ => None,
        }
    }

    fn peek_interface(&self) -> Option<Interface> {
        match self.state {
            ParserStateMachine::Config(index) | ParserStateMachine::Inetrface(index, _) => {
                trace!(
                    "peek at {},value:{}",
                    self.current,
                    self.configs[index].0[self.current]
                );

                if self.peek_std_desc_type() == Some(USBStandardDescriptorTypes::Interface) {
                    let len = self.configs[index].0[self.current] as usize;
                    let from = self.current;
                    let to = from + len - 1;
                    trace!("len{len},from{from},to{to}");
                    let raw = (&self.configs[index].0[from..to]) as *const [u8];
                    let interface = unsafe { ptr::read_volatile(raw as *const Interface) }; //do not cast, in current version rust still had value cache issue
                    trace!("got:{:?}", interface);

                    return Some(interface);
                }
            }
            _ => {}
        }
        None
    }

    fn parse_interface(&mut self) -> Result<Interface, Error> {
        trace!("parse interfaces,metadata:{:?}", self.metadata);
        match self.parse_any_descriptor()? {
            USBDescriptor::Interface(int) => {
                match &self.metadata {
                    ParserMetaData::UVC(_) => {
                        self.metadata = ParserMetaData::UVC(int.interface_subclass.clone());
                    }
                    ParserMetaData::Unknown(ParserMetaDataUnknownSituation::ReferInterface) => {
                        self.metadata = ParserMetaData::determine(
                            int.interface_class,
                            int.interface_subclass,
                            int.interface_protocol,
                        )
                    }
                    _ => {}
                }
                Ok(int)
            }
            _ => Err(Error::ParseOrderError),
        }
    }

    fn parse_other_descriptors_by_metadata(&mut self) -> Vec<USBDescriptor> {
        trace!(
            "parse additional data for interface with metadata:{:?}",
            self.metadata
        );
        let mut vec = Vec::new();
        loop {
            match self.peek_std_desc_type() {
                Some(
                    USBStandardDescriptorTypes::Endpoint
                    | USBStandardDescriptorTypes::Interface
                    | USBStandardDescriptorTypes::InterfaceAssociation,
                ) => break,
                Some(_) | None => {
                    trace!("parse misc desc!");
                    vec.push(
                        self.parse_any_descriptor()
                            .inspect_err(|e| error!("usb descriptor parse failed:{:?}", e))
                            .unwrap(),
                    );
                    continue;
                }
            }
        }
        vec
    }

    fn parse_endpoints(&mut self) -> Vec<TopologicalUSBDescriptorEndpoint> {
        trace!("parse enedpoints, metadata:{:?}", self.metadata);
        let mut endpoints = Vec::new();

        loop {
            if let Some(USBStandardDescriptorTypes::Endpoint) = self.peek_std_desc_type() {
                match self.parse_any_descriptor().unwrap() {
                    USBDescriptor::Endpoint(endpoint) => {
                        trace!("parsed endpoint:{:?}", endpoint);
                        endpoints.push(TopologicalUSBDescriptorEndpoint::Standard(endpoint))
                    }
                    _ => {}
                }
                continue;
            }

            match self.metadata {
                ParserMetaData::UVC(_) => {
                    if let Some(UVCDescriptorTypes::UVCClassSpecVideoControlInterruptEndpoint) =
                        self.peek_uvc_desc_type()
                    {
                        trace!("uvc interrupt endpoint!");
                        match self.parse_any_descriptor().unwrap() {
                            USBDescriptor::UVCClassSpecVideoControlInterruptEndpoint(ep) => {
                                trace!("got {:?}", ep);
                                endpoints.push(TopologicalUSBDescriptorEndpoint::UNVVideoControlInterruptEndpoint(ep));
                            }
                            _ => {
                                panic!("impossible!");
                            }
                        }
                        continue;
                    } else {
                        trace!("not uvc data!");
                    }
                }
                _ => {}
            }

            break;
        }
        endpoints
    }
}
