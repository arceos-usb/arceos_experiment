#![no_std]
#![no_main]
use axhal::time::busy_wait;
use core::default;
use core::ptr;
use core::slice;
use core::time::Duration;
use log::*;

use crate::driver_iic::i2c::*;
use crate::driver_iic::i2c_hw::*;
use crate::driver_iic::i2c_intr::*;
use crate::driver_iic::i2c_master::*;
use crate::driver_iic::i2c_sinit::*;
use crate::driver_iic::io::*;
use crate::driver_iic::*;

use crate::driver_mio::mio::*;
use crate::driver_mio::mio_g::*;
use crate::driver_mio::mio_hw::*;
use crate::driver_mio::mio_sinit::*;

// 定义FI2cSlaveData结构体
#[derive(Debug, Clone, Copy)]
struct FI2cSlaveData {
    pub device: FI2c,
    pub first_write: bool,
    pub buff_idx: u32,
    pub buff: [u8; 256],
}

pub static mut slave:FI2cSlaveData = FI2cSlaveData{
    device:FI2c{
        config:FI2cConfig {
            instance_id: 0,                
            base_addr: 0,      
            irq_num: 0,                  
            irq_priority: 0,               
            ref_clk_hz: 0,         
            work_mode: 0,                  
            slave_addr: 0,                
            use_7bit_addr: false,             
            speed_rate: 0,           
        },
        is_ready:0,
        status:0,
        txframe:FI2cFrameTX{
            data_buff: core::ptr::null_mut(), 
            tx_total_num: 0,                  
            tx_cnt: 0,                        
            flag: 0,                        
        },
        rxframe:FI2cFrameRX {
            data_buff: core::ptr::null_mut(),  
            rx_total_num: 0,                
            rx_cnt: 0,                    
        },
        master_evt_handlers:[None;3],
        slave_evt_handlers:[None;6],
    },
    first_write:false,
    buff_idx:0,
    buff:[0;256],
};


impl Default for FI2cSlaveData {
    fn default() -> Self {
        Self {
            device: FI2c::default(),
            first_write: false,
            buff_idx: 0,
            buff: [0; 256],
        }
    }
}

// 定义FI2cSlaveCb函数
pub fn FI2cSlaveCb(instance_p: *mut core::ffi::c_void, para: *mut core::ffi::c_void, evt: u32) {
    let mut slave_p = FI2cSlaveData::default();
    let val = para as *mut u8;

    if slave_p.buff_idx >= 256 as u32 {
        slave_p.buff_idx %= 256 as u32;
    }

    match evt {
        3 => {
            if slave_p.first_write {
                slave_p.buff_idx = unsafe { *val as u32 };
                slave_p.first_write = false;
            } else {
                slave_p.buff[slave_p.buff_idx as usize] = unsafe { *val };
                slave_p.buff_idx += 1;
            }
        }
        2 => {
            slave_p.buff_idx += 1;
        }
        0 => {
            unsafe { *val = slave_p.buff[slave_p.buff_idx as usize] };
            slave_p.buff_idx += 1;
        }
        4 | 1 => {
            slave_p.first_write = true;
        }
        5 => {
            // 处理中断
        }
        _ => {}
    }
}

// 定义其他中断处理函数
pub fn FI2cSlaveWriteReceived(instance_p: *mut core::ffi::c_void, para: *mut core::ffi::c_void) {
    FI2cSlaveCb(instance_p, para, 3);
}

pub fn FI2cSlaveReadProcessed(instance_p: *mut core::ffi::c_void, para: *mut core::ffi::c_void) {
    FI2cSlaveCb(instance_p, para, 2);
}

pub fn FI2cSlaveReadRequest(instance_p: *mut core::ffi::c_void, para: *mut core::ffi::c_void) {
    FI2cSlaveCb(instance_p, para, 0);
}

pub fn FI2cSlaveStop(instance_p: *mut core::ffi::c_void, para: *mut core::ffi::c_void) {
    FI2cSlaveCb(instance_p, para, 4);
}

pub fn FI2cSlaveWriteRequest(instance_p: *mut core::ffi::c_void, para: *mut core::ffi::c_void) {
    FI2cSlaveCb(instance_p, para, 1);
}

pub fn FI2cSlaveAbort(instance_p: *mut core::ffi::c_void, para: *mut core::ffi::c_void) {
    FI2cSlaveCb(instance_p, para, 5);
}

// pub fn FI2cMioSlaveInit(address: u32, speed_rate: u32) -> bool {
//     let mut input_cfg:FI2cConfig;
//     let mut slave_p:FI2cSlaveData = FI2cSlaveData_new();
//     let mut instance_p:FI2c = slave.device;
//     let mut status:bool = true;

//     // 初始化 MIO
//     let mut slave_mio_ctrl:FMioCtrl;
//     slave_mio_ctrl.config = FMioLookupConfig(1).unwrap();
//     status = FMioFuncInit(&mut slave_mio_ctrl, 0b00);
//     if status != true {
//         trace!("MIO initialize error.");
//         return false;
//     }

//     // 初始化 MIO 功能
//     FIOPadSetFunc(&instance_p, 0x00D0U, 5); /* scl */
//     FIOPadSetFunc(&instance_p, 0x00D4U, 5); /* sda */
//     core::ptr::write_bytes(slave_p as *mut FI2cSlaveData, 0, size_of::<FI2cSlaveData>());
//     slave_p.first_write = true;

//     // 查找默认配置
//     let config_p = FI2cLookupConfig(0);
//     if config_p.is_none() {
//         trace!("Config of mio instance {} non found.", 1);
//         return false;
//     }
//     let config_p = config_p.unwrap();

//     // 修改配置
//     input_cfg = *config_p;
//     input_cfg.instance_id = 1;
//     input_cfg.base_addr = FMioFuncGetAddress(&slave_mio_ctrl, 0b00);
//     input_cfg.irq_num = FMioFuncGetIrqNum(&slave_mio_ctrl, 0b00);
//     input_cfg.ref_clk_hz = 50000000;
//     input_cfg.work_mode = 1;
//     input_cfg.slave_addr = address;
//     input_cfg.speed_rate = speed_rate;

//     // 初始化 I2C
//     let mut instance_p = &mut slave_p.device;
//     status = FI2cCfgInitialize(instance_p, &input_cfg);
//     if status != true {
//         trace!("Init mio slave failed, ret: 0x{:x}", status);
//         return status;
//     }

//     // 注册中断处理程序
//     FI2cSlaveRegisterIntrHandler(instance_p, 3, FI2cSlaveWriteReceived);
//     FI2cSlaveRegisterIntrHandler(instance_p, 2, FI2cSlaveReadProcessed);
//     FI2cSlaveRegisterIntrHandler(instance_p, 0, FI2cSlaveReadRequest);
//     FI2cSlaveRegisterIntrHandler(instance_p, 4, FI2cSlaveStop);
//     FI2cSlaveRegisterIntrHandler(instance_p, 1, FI2cSlaveWriteRequest);
//     FI2cSlaveRegisterIntrHandler(instance_p, 5, FI2cSlaveAbort);

//     // 设置中断
//     let cpu_id = get_cpu_id();
//     interrupt_set_target_cpus(input_cfg.irq_num, cpu_id);
//     interrupt_set_priority(input_cfg.irq_num, input_cfg.irq_priority);
//     interrupt_install(input_cfg.irq_num, fi2c_slave_intr_handler, instance_p, "fi2cslave");

//     // 配置 I2C 从机中断
//     status = fi2c_slave_setup_intr(instance_p);
//     interrupt_umask(input_cfg.irq_num);

//     if status != FError::SUCCESS {
//         error!("Setup mio slave interrupt failed, ret: 0x{:x}", status);
//         return status;
//     }
//     true
// }

pub unsafe  fn FI2cMioMasterInit(address: u32, speed_rate: u32) -> bool {
    let mut input_cfg: FI2cConfig = FI2cConfig::default();
    let mut config_p: FI2cConfig = FI2cConfig::default();
    let mut status: bool = true;

    // MIO 初始化
    master_mio_ctrl.config = FMioLookupConfig(1).unwrap();
    status = FMioFuncInit(&mut master_mio_ctrl, 0b00);
    if status != true {
        trace!("MIO initialize error.");
        return false;
    }
    FIOPadSetFunc(&iopad_ctrl, 0x00D0u32, 5); /* scl */
    FIOPadSetFunc(&iopad_ctrl, 0x00D4u32, 5); /* sda */

    unsafe {
        core::ptr::write_bytes(&mut master_i2c_instance as *mut FI2c, 0, size_of::<FI2c>());
    }
    // 查找默认配置
    config_p = FI2cLookupConfig(1).unwrap(); // 获取 MIO 配置的默认引用
    if !Some(config_p).is_some() {
        trace!("Config of mio instance {} not found.", 1);
        return false;
    }

    // 修改配置
    input_cfg = config_p.clone();
    input_cfg.instance_id = 1;
    input_cfg.base_addr = FMioFuncGetAddress(&master_mio_ctrl, 0b00);
    input_cfg.irq_num = FMioFuncGetIrqNum(&master_mio_ctrl, 0b00);
    input_cfg.ref_clk_hz = 50000000;
    input_cfg.slave_addr = address;
    input_cfg.speed_rate = speed_rate;

    // 初始化
    status = FI2cCfgInitialize(&mut master_i2c_instance, &input_cfg);

    // 处理 FI2C_MASTER_INTR_EVT 中断的回调函数
    master_i2c_instance.master_evt_handlers[0 as usize] = None;
    master_i2c_instance.master_evt_handlers[1 as usize] = None;
    master_i2c_instance.master_evt_handlers[2 as usize] = None;

    if status != true {
        trace!("Init mio master failed, ret: {:?}", status);
        return status;
    }

    trace!(
        "Set target slave_addr: 0x{:x} with mio-{}",
        input_cfg.slave_addr, 1
    );
    status
}

pub unsafe fn FI2cMasterWrite(buf_p: &mut [u8], buf_len: u32, inchip_offset: u32) -> bool {
    let mut status: bool = true;

    if buf_len < 256 && inchip_offset < 256 {
        if (256 - inchip_offset) < buf_len {
            trace!("Write to eeprom failed, out of eeprom size.");
            return false;
        }
    } else {
        trace!("Write to eeprom failed, out of eeprom size.",);
        return false;
    }

    status = FI2cMasterWritePoll(&mut master_i2c_instance, inchip_offset, 1, buf_p, buf_len);
    trace!("++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++");
    if status != true {
        trace!("Write to eeprom failed");
    }

    status
}

pub unsafe fn FI2cMasterRead(buf_p: &mut [u8], buf_len: u32, inchip_offset: u32) -> bool {
    let mut instance_p: FI2c = master_i2c_instance;
    let mut status: bool = true;

    assert!(buf_len != 0);

    for i in 0..buf_len as usize {
        buf_p[i] = 0;
    }

    status = FI2cMasterReadPoll(&mut instance_p, inchip_offset, 1, buf_p, buf_len);

    status
}

pub fn FtDumpHexByte(ptr: *const u8, buflen: usize) {
    unsafe {
        let buf = unsafe { slice::from_raw_parts(ptr, buflen) };
        for i in (0..buflen).step_by(16) {
            trace!("{:p}: ", ptr.add(i));
            for j in 0..16 {
                if i + j < buflen {
                    trace!("{:02X} ", buf[i + j]);
                } else {
                    trace!("   ");
                }
            }
            trace!(" ");
            for j in 0..16 {
                if i + j < buflen {
                    let c = buf[i + j] as char;
                    if c.is_ascii_graphic() {
                        trace!("{}", c);
                    } else {
                        trace!(".");
                    }
                }
            }
            trace!("");
        }
    }
}

pub unsafe fn FI2cSlaveDump() {
    let mut slave_p = slave;
    trace!(
        "buf size: {}, buf idx: {}",
        slave_p.buff.len(),
        slave_p.buff_idx
    );
    FtDumpHexByte(slave_p.buff.as_ptr(), 256);
}