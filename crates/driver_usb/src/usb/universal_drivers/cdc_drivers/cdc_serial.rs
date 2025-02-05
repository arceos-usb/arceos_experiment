use alloc::string::String;
use alloc::boxed::Box;
use alloc::{sync::Arc, vec, vec::Vec};
use alloc::collections::VecDeque;
use axalloc::PAGE_SIZE;
//todo!("is this ok?")
use spinlock::SpinNoIrq;
use log::trace;
use crate::{
    abstractions::{PlatformAbstractions,dma::DMA},
    glue::{
        driver_independent_device_instance::DriverIndependentDeviceInstance,
        ucb::{CompleteCode, TransferEventCompleteCode, UCB},
    },
    host::data_structures::MightBeInited,
    usb::{
        descriptors::{
            desc_device::StandardUSBDeviceClassCode,
            desc_endpoint::Endpoint,
            topological_desc::{TopologicalUSBDescriptorEndpoint, TopologicalUSBDescriptorFunction},
        },
        drivers::driverapi::{USBSystemDriverModule, USBSystemDriverModuleInstance},
        trasnfer::{
            control::{bRequest, bmRequestType, ControlTransfer, DataTransferType, Recipient},
            bulk::BulkTransfer,  
        },//todo!("transfer or trasnfer?")
        urb::{RequestedOperation, URB},
    },
    USBSystemConfig,
};
use xhci::ring::trb::transfer::Direction;

pub struct CdcSerialDriver<O>
where
    O: PlatformAbstractions,
{
    device_slot_id: usize,
    in_endpoint: u32, // 输入端点
    out_endpoint: u32, // 输出端点
    config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    interface_value: usize,
    config_value: usize,
    read_urb: URB<'static, O>, // 使用生命周期参数 'static
    read_data_buffer: Option<SpinNoIrq<DMA<[u8], O::DMA>>>, // 存放读取数据
    accept_accepted_data: Vec<u8>, // 存放已读取数据
    urb_buffer: VecDeque<Box<URB<'static, O>>>, // 存放URB
    write_data_buffer: VecDeque<Box<SpinNoIrq<DMA<[u8], O::DMA>>>>, // 存放写入数据
    last_urb: Option<URB<'static, O>>, // 存放前一个提交的URB，用作状态机
}

impl<O> CdcSerialDriver<O>
where
    O: PlatformAbstractions + 'static,
{
    pub fn new_and_init(
        device_slot_id: usize,
        endpoints: Vec<Endpoint>,
        config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
        interface_value: usize,
        config_value: usize,
    ) -> Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'static, O>>> {
        trace!("CdcSerialDriver initializing");
        trace!("endpoints: {:?}", endpoints);
        let rb = DMA::new_vec(
            0u8,
            O::PAGE_SIZE,
            O::PAGE_SIZE,
            config.lock().os.dma_alloc(),
        );
        // 端点地址最高位为1表示输入端点，最低位为0表示输出端点。属性为0x02表示端点类型为Bulk。
        trace!(
            "in_endpoint对应的端点: {:?}", 
            endpoints
            .iter()
            .find(|e| e.endpoint_address & 0x1000_0000 != 0 && e.attributes == 0x02)
            .unwrap()
            .clone()
        );
        trace!(
            "out_endpoint对应的端点: {:?}",             
            endpoints.iter()
            .find(|e| e.endpoint_address&0x1000_0000 ==0&&e.attributes==0x02)
            .unwrap()
            .clone());
        Arc::new(SpinNoIrq::new(Self {
                device_slot_id,
                in_endpoint: endpoints.iter().find(|e| e.endpoint_address & 0x1000_0000 != 0 && e.attributes == 0x02).unwrap().clone().doorbell_value_aka_dci(),
                out_endpoint: endpoints.iter().find(|e| e.endpoint_address & 0x1000_0000 == 0 && e.attributes == 0x02).unwrap().clone().doorbell_value_aka_dci(),
                config,
                interface_value,
                config_value,
                read_urb: URB::new(
                    device_slot_id,
                    RequestedOperation::Bulk(BulkTransfer {
                        endpoint_id: endpoints.iter().find(|e| e.endpoint_address & 0x1000_0000 != 0 && e.attributes == 0x02).unwrap().clone().doorbell_value_aka_dci() as usize,
                        buffer_addr_len: rb.addr_len_tuple(),
                    }),
                ),
                read_data_buffer: Some(SpinNoIrq::new(rb)),
                accept_accepted_data: Vec::new(),
                urb_buffer: VecDeque::new(),
                write_data_buffer: VecDeque::new(),
                last_urb:None,
                }
            )
        )
    }
    pub fn write(){}
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

        //todo!("设置buffer");
        let buffer = DMA::new_vec(
            0u8,
            2,
            2,
            self.config.lock().os.dma_alloc(),
        );

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
                data: None, //todo!
                response: true,
            }),
        ));
        //厂商自定义的配置流程，参考ch341.c的ch341_configure函数

        //第一个控制输入传输：CMD_C3
        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::In,
                    DataTransferType::Vendor,
                    Recipient::Device,
                ),
                request: bRequest::CH341_CMD_C3,
                index: 0,
                value: 0,
                data: Some((buffer.addr_len_tuple())), // 传递数据缓冲区地址和大小
                response: true,
            }),
        ));

        // 第一个控制输出传输：CMD_C1
        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::Out,
                    DataTransferType::Vendor,
                    Recipient::Device,
                ),
                request: bRequest::CH341_CMD_C1,
                index: 0,
                value: 0,
                data: None,
                response: false,
            }),
        ));

        // 第二个控制输出传输：CMD_W, 0x1312, 0xd982
        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::Out,
                    DataTransferType::Vendor,
                    Recipient::Device,
                ),
                request: bRequest::CH341_CMD_W,
                index: 0xd982,
                value: 0x1312,
                data: None,
                response: false,
            }),
        ));

        // 第三个控制输出传输：CMD_W, 0x0f2c, 0x0007
        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::Out,
                    DataTransferType::Vendor,
                    Recipient::Device,
                ),
                request: bRequest::CH341_CMD_W,
                index: 0x0007,
                value: 0x0f2c,
                data: None,
                response: false,
            }),
        ));

        // 第二个控制输入传输：CMD_R, 0x2518
        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::In,
                    DataTransferType::Vendor,
                    Recipient::Device,
                ),
                request: bRequest::CH341_CMD_R,
                index: 0,
                value: 0x2518,
                data: Some((buffer.addr_len_tuple())), // 传递数据缓冲区地址和大小
                response: true,
            }),
        ));

        // todo!("get status");

        // 第四个控制输出传输：CMD_W, 0x2727, 0
        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::Out,
                    DataTransferType::Vendor,
                    Recipient::Device,
                ),
                request: bRequest::CH341_CMD_W,
                index: 0,
                value: 0x2727,
                data: None,
                response: false,
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
