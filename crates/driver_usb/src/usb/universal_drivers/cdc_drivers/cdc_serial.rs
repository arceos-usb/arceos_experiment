use alloc::{
    boxed::Box,
    collections::VecDeque,
    string::String,
    sync::Arc,
    vec,
    vec::Vec,
};
use axalloc::PAGE_SIZE;
use spinlock::SpinNoIrq;
use log::trace;
use xhci::{
    extended_capabilities::debug::Status,
    ring::trb::transfer::Direction,
};
use crate::{
    abstractions::{PlatformAbstractions, dma::DMA},
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
        },
        urb::{RequestedOperation, URB},
    },
    USBSystemConfig,
};

#[derive(Debug)]
pub enum state_machine {
    Waiting,
    Writing,
    Reading,
}

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
    driver_state_machine: state_machine,
    ctl_buffer: Option<SpinNoIrq<DMA<[u8], O::DMA>>>, // 控制传输缓冲区
    status_buffer: Option<SpinNoIrq<DMA<[u8], O::DMA>>>, // 状态缓冲区
    read_data_buffer: Option<SpinNoIrq<DMA<[u8], O::DMA>>>, // 存放读取数据
    accepted_data: Vec<u8>, // 存放已读取数据
    write_data_buffer: VecDeque<Box<SpinNoIrq<DMA<[u8], O::DMA>>>>, // 存放写入数据
}

impl<'a,O> CdcSerialDriver<O>
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
        //读缓冲区，按照读取端点的最大包大小分配
        let mut rb = DMA::new_vec(
            0u8,
            32,
            O::PAGE_SIZE,
            config.lock().os.dma_alloc(),
        );
        // 端点地址最高位为1表示输入端点，最低位为0表示输出端点。属性为0x02表示端点类型为Bulk。
        trace!(
            "in_endpoint对应的端点: {:?}", 
            endpoints
            .iter()
            .find(|e| e.endpoint_address & 0b1000_0000 != 0 && e.attributes == 0x02)
            .unwrap()
            .clone()
        );
        trace!(
            "out_endpoint对应的端点: {:?}",             
            endpoints.iter()
            .find(|e| e.endpoint_address&0b1000_0000 ==0&&e.attributes==0x02)
            .unwrap()
            .clone());
        //todo!("添加一个用于测试的写缓冲区成员")
        let mut write_buf = DMA::new_vec(
            0u8,
            11,
            O::PAGE_SIZE,
            config.lock().os.dma_alloc(),
        );
        let write_info:&[u8] = b"hello,world";
        let mut write_buf_mut = &mut *write_buf;
        write_buf_mut.copy_from_slice(write_info);
        trace!("-------------------------------------------------------------------------------------");
        trace!("write_buf: {:?}", write_buf_mut);
        trace!("-------------------------------------------------------------------------------------");
        let mut write_data_buffer = VecDeque::new();
        write_data_buffer.push_back(Box::new(SpinNoIrq::new(write_buf)));
        //todo!("添加一个用于测试的写缓冲区成员")
        let mut ctl_buf = DMA::new_vec(
            0u8,
            2,
            2,
            config.lock().os.dma_alloc(),
        );
        let mut status_buf = DMA::new_vec(
            0u8,
            2,
            2,
            config.lock().os.dma_alloc(),
        );
        Arc::new(SpinNoIrq::new(Self {
                device_slot_id,
                in_endpoint: endpoints.iter().find(|e| e.endpoint_address & 0b1000_0000 != 0 && e.attributes == 0x02).unwrap().clone().doorbell_value_aka_dci(),
                out_endpoint: endpoints.iter().find(|e| e.endpoint_address & 0b1000_0000 == 0 && e.attributes == 0x02).unwrap().clone().doorbell_value_aka_dci(),
                config,
                interface_value,
                config_value,
                ctl_buffer: Some(SpinNoIrq::new(ctl_buf)),
                status_buffer: Some(SpinNoIrq::new(status_buf)),
                read_data_buffer: Some(SpinNoIrq::new(rb)),
                driver_state_machine : state_machine::Waiting,
                accepted_data: Vec::new(),
                write_data_buffer: write_data_buffer,
                }
            )
        )
    }
    pub fn write(&mut self, data: &[u8]) {
        //把一个字节序列的数据写入到写入数据缓冲区
        let mut buffer = DMA::new_vec(
            1u8,
            data.len(),
            data.len(),
            self.config.lock().os.dma_alloc(),
        );
        let mut buffer_mut = &mut *buffer;
        buffer_mut.copy_from_slice(data);
        self.write_data_buffer.push_back(Box::new(SpinNoIrq::new(buffer)));
    }
}

impl<'a, O> USBSystemDriverModuleInstance<'a, O> for CdcSerialDriver<O>
where
    O: PlatformAbstractions,
{
    fn gather_urb(&mut self) -> Option<Vec<crate::usb::urb::URB<'a, O>>> {
        //总的逻辑是遍历写入数据缓冲区，如果有数据就发送，如果没有数据就接收
        let mut todo_list = Vec::new();
        if self.write_data_buffer.len() > 0 {
            //如果写入数据缓冲区有数据，就发送数据
            let write_buffer = if let Some(buffer) = self.write_data_buffer.front() {
                buffer
            } else {
                panic!("write_data_buffer is empty, but write_data_buffer.len() > 0");
            };
            todo_list.push(URB::new(
                self.device_slot_id,
                RequestedOperation::Bulk(BulkTransfer {
                    endpoint_id: self.out_endpoint as usize,
                    buffer_addr_len: write_buffer.lock().addr_len_tuple(),
                }),
            ));
            self.driver_state_machine = state_machine::Writing;
            Some(todo_list)
        } else {
            //如果写入数据缓冲区没有数据，就接收数据
            todo_list.push(URB::new(
                self.device_slot_id,
                RequestedOperation::Bulk(BulkTransfer {
                    endpoint_id: self.in_endpoint as usize,
                    buffer_addr_len: self.read_data_buffer.as_ref().unwrap().lock().addr_len_tuple(),
                }),
            ));
            self.driver_state_machine = state_machine::Reading;
            Some(todo_list)
        }
    }
    fn receive_complete_event(&mut self, ucb: UCB<O>) {
        //结合UCB和状态机处理接收完成事件
        match ucb.code {
            CompleteCode::Event(TransferEventCompleteCode::Success) => {
                match self.driver_state_machine {
                    //成功读取本来应该把数据传递给其他子系统，这里暂时存放在accepted_data中
                    //测试时存放的都是字符，所以00是结束符
                    state_machine::Reading => {
                        trace!("-------------------------------------------------------------------------------------");
                        trace!("reading data");
                        let rec_data = 
                                self.read_data_buffer
                                .as_ref()
                                .unwrap()
                                .lock()
                                .as_mut()
                                .iter()
                                .filter_map(|b| if *b != 0u8 { Some(*b) } else { None })
                                .collect::<Vec<u8>>();
                        let mut s0 = String::new();
                        for b in rec_data.iter() {
                            s0.push(*b as char);
                        }
                        trace!("received data: {:?}", s0);
                        self.read_data_buffer
                            .as_ref()
                            .unwrap()
                            .lock()
                            .as_mut()
                            .iter()
                            .for_each(|b| 
                                {
                                    if *b != 0u8 {
                                        self.accepted_data.push(*b);
                                    }
                                });
                        //将accepted_data中的数据转换成字符串
                        let mut s = String::new();
                        for b in self.accepted_data.iter() {
                            s.push(*b as char);
                        }
                        trace!("accepted data: {:?}", s);
                        trace!("-------------------------------------------------------------------------------------");

                        self.driver_state_machine = state_machine::Waiting;
                    }
                    state_machine::Writing => {
                        self.write_data_buffer.pop_front();//发送成功后，删除发送缓冲区
                        trace!("writing data completed");
                        self.driver_state_machine = state_machine::Waiting;
                    }
                    _ => {
                        trace!("received success event in waiting state");
                        trace!("status is {:?}", self.status_buffer.as_ref().unwrap().lock().as_mut());
                        trace!("ctl is {:?}", self.ctl_buffer.as_ref().unwrap().lock().as_mut());
                    }
                }
            }
            CompleteCode::Event(TransferEventCompleteCode::Babble) => {
                trace!("received babble event,this state is {:?}", self.driver_state_machine);
            }
            other => trace!("received other event: {:?},should be control completecodes ", other),
        }
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
                data: Some((self.ctl_buffer.as_ref().unwrap().lock().addr_len_tuple())), // 传递数据缓冲区地址和大小
                response: false,
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
                response: true,
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
                response: true,
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
                response: true,
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
                data: Some((self.ctl_buffer.as_ref().unwrap().lock().addr_len_tuple())), // 传递数据缓冲区地址和大小
                response: false,
            }),
        ));

        // todo!("get status");
        // 控制输入传输，获取状态
        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::In,
                    DataTransferType::Vendor,
                    Recipient::Device,
                ),
                request: bRequest::CH341_CMD_R,
                index: 0x0706,
                value: 0,
                data: Some((self.status_buffer.as_ref().unwrap().lock().addr_len_tuple())),
                response: false,
            }),
        ));
        // 抓Linux得到的状态是ffee
        // 我收到的是171b
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
                response: true,
            }),
        ));

        //下面wireshark抓包发现的控制传输
        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::Out,
                    DataTransferType::Vendor,
                    Recipient::Device,
                ),
                request: bRequest::CH341_CMD_C1,
                index: 0xb282,
                value: 0xc39c,
                data: None,
                response: true,
            }),
        ));

        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::Out,
                    DataTransferType::Vendor,
                    Recipient::Device,
                ),
                request: bRequest::CH341_CMD_W,
                index: 0x0008,
                value: 0x0f2c,
                data: None,
                response: true,
            }),
        ));

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
