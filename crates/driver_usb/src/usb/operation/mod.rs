use super::descriptors::TopologicalUSBDescriptorConfiguration;

#[derive(Debug, Clone)]
pub enum Configuration<'a> {
    SetupDevice(&'a TopologicalUSBDescriptorConfiguration),
    SwitchInterface(InterfaceNumber, AltnativeNumber),
}

pub type ConfigurationID = usize;
pub type InterfaceNumber = usize;
pub type AltnativeNumber = usize;