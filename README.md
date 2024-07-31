# proj2210132-基于飞腾派的Arceos移植与外设驱动开发-设计文档

* 队名：旋转轮椅
* 成员：姚宏伟，郭天宇，蒋秋吉
* 学校：上海建桥学院

[TOC]

## 如何运行复现？这里是使用手册

使用手册遵循Arceos开发的惯例，其位于[doc目录](./doc)下，其中：

* [USB系统使用样例](./doc/apps_usb-hid.md)

## 目标概述

飞腾派开发板是飞腾公司针对教育行业推出的一款国产开源硬件平台，兼容 ARMv8-A 处理器架构。飞腾小车是一款利用飞腾派开发板组装的小车，它是一个计算机编程实验平台，包含电机驱动、红外传感器、超声传感器、AI加速棒、视觉摄像云台等，可以满足学生学习计算机编程、人工智能训练等课题。

ArceOS是基于组件化设计的思路，用Rust语言的丰富语言特征，设计实现不同功能的独立操作系统内核模块和操作系统框架，可形成不同特征/形态/架构的操作系统内核。

![arceos](./doc/figures/ArceOS.svg)

本项目需要参赛队伍**将ArceOS移植到飞腾派开发板**，实现**UART串口通信**、**以太网口通信（HTTP server和HTTP client 可以上传下载文件）**，并且可以通过实现**I2C驱动**和**USB驱动**去驱动小车行走。

* 完成ArceOS操作系统移植到飞腾派开发板。
* 实现UART串口通信。
* 实现I2C驱动-用于驱动小车电机。
* **实现USB驱动，并接受外部输入以控制小车**

## 完成概况

1. 系统移植：完成

2. UART串口通信：完成

3. i2c驱动：已跑通，尚未在小车上的驱动板进行测试

4. USB驱动：已完成，但情况较为复杂，详见下文开发进展情况说明中的USB驱动部分
   5.1.  XHCI主机驱动：完成
   
      5.1.1. 通用性usb主机端驱动框架重构：完成
   
   5.2.  USB-HID：完成，已有鼠标测试用例

## 在一切开始前，前人所作的工作&开发进展情况说明

本仓库基于[该分支](https://github.com/limingth/arceos)进行开发

* 飞腾派所使用的UART控制器为[PL011](https://developer.arm.com/documentation/ddi0183/latest/)，是一款被广泛应用的适用于Arm平台的芯片，因此，该芯片的驱动有较多实例可供参考。
* i2c驱动：无，逆向官方的sdk，从0开始开发。
* USB驱动：本任务中所指的USB驱动，是指USB在主机端的一整套子系统，就从软件层面来说，其包含[XHCI(USB3.0主机控制器)](./doc/resources/[XHCI控制器规范]extensible-host-controler-interface-usb-xhci.pdf)驱动与[USB协议层驱动](./doc/resources/[USB3.2协议规范]USB%203.2%20Revision%201.1.pdf)
  * XHCI驱动：分为两部分，首先是主机本身的控制程序，然后是基于XHCI的USB设备枚举程序
    * 主机控制程序: 参见XHCI规范第四章(Operation Model),这部分主要负责主机寄存器的相关操作，与硬件关联的数据结构的管理(如控制器在内存中的抽象，控制器所使用的内存的管理(TRB环，中断向量等))
    * 设备枚举程序: 在主机控制程序完成后，我们还需要根据USB规范，对所连接的设备进行一次扫描，扫描过程中需要对设备做对应的初始化（这部分是通用操作，包含启用USB端点，选择设备配置等）
  * USB协议层：USB规范是类似于OSI网络七层架构一样的设计，换句话来说，USB规范并不是单本规范，而是**一套**规范，在USB所定义的四种传输类型（传输层）的基础上，则是随着USB设备类型的日益增加而变的[无比庞大](https://www.usbzh.com/article/special.html)的协议层，举几个例子：
    * [HID(Human Interface Device)协议](./doc/resources/HID/hid1_11.pdf)指的是人类交互设备，如：鼠标，键盘，手柄等设备。
    * [UVC(USB video Class)协议](./doc/resources/[UVC]/UVC%201.5%20Class%20specification.pdf)是是微软与另外几家设备厂商联合推出的为USB视频捕获设备定义的协议标准
    * ...
  * 在更之上，还有诸多厂家自定义的协议，因此，在我们从0开始，创新性的使用RUST编写操作系统的情况下，想在这么短的时间内，凭借不多的人手去移植整个USB系统是几乎不可能的，因此我们选择，**从协议层开始，先仅仅移植出一套可以形成控制闭环的最小系统，并且在编写的过程中做好整个USB子系统的设计，以方便日后的来者进行开发**
  * 从另一方面来说，用Rust所编写的完整的USB主机驱动也很少，因此可以拿来参考的案例也比较少，Redox算是一个，但是其usb系统和核心部分分别位于用户态和内核态，参考意义不大

## 工作日志

### 初赛

#### 第一阶段-系统移植&串口移植-2024.3-2024.4

| 时间节点      | 里程碑                                                 |
| --------- | --------------------------------------------------- |
| 2024/3/27 | 收到设备，初步配置开发环境                                       |
| 2024/3/29 | dump出了内存布局，新建并修改了arceos的平台描述文档使其适配平台的内存结构           |
| 2024/3/30 | 开始串口调试，进入Uboot，确认路线：通过uboot(tftp加载)引导arceos镜像，并实验成功 |
| 2024/4/3  | 系统移植成功，能够通过板子上的gpio进行串口通信，可以与系统的命令行交互               |

#### 第二阶段-USB系统移植-跑通前

| 时间节点      | 里程碑-USB系统                           | 里程碑-i2c驱动       |
| --------- | ----------------------------------- | --------------- |
| 2024/4/7  | 收到飞腾小车，开始研究小车的结构，同时开始抽时间研究i2c驱动     |                 |
| 2024/4/9  | 开始编写usb系统的驱动                        |                 |
| 2024/5/1  | Xhci控制器初始化完成                        |                 |
| 2024/5/15 | 成功通过控制器给设备分配了地址（即-第一条xhci控制命令的成功发送） |                 |
| 2024/5/17 |                                     | 编译出了官方sdk中的demo |
| 2024/6/15 | 开启了设备的控制端点，能够进行控制传输，并编写了初赛所需要的文档    |                 |

### 决赛

#### 第二阶段-USB系统移植-跑通

| 时间节点      | 里程碑-USB系统                                  | 里程碑-i2c驱动           |
| --------- | ------------------------------------------ | ------------------- |
| 2024/6/20 | 获取到了设备的描述符，并简单编写了相应的描述符解析代码                |                     |
| 2024/7/10 | 成功根据设备的端点配置好了设备的其余通信断电，决定先写个hid驱动          |                     |
| 2024/7/15 | 鼠标驱动demo大体完成，能够获取单次回报数据，开始检修bug            | 开始根据sdk编写i2c驱动以驱动小车 |
| 2024/7/18 | 经排查，定位到bug问题出现在传输环的翻转位上，经过修复后可以正常建立有线鼠标的通信 |                     |
| 2024/7/20 | 成功编写出无线鼠标的驱动（即-实现了复合hid设备）                 |                     |

#### 第三阶段-提供跨平台/跨操作系统移植友好性的usb驱动框架-重构&uvc摄像头驱动开发

| 时间节点      | 里程碑-USB系统                                                                                                                                                                                                                                               | 里程碑-i2c驱动      |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| 2024/7/22 | 经研究与实际需求相结合，决定开始编写uvc摄像头驱动                                                                                                                                                                                                                              |                |
| 2024/7/23 | 已经到了不得不重构的时候了，开始重构整个usb框架                                                                                                                                                                                                                               |                |
| 2024/7/29 | 完成了框架的重构，将原有的hid驱动搬到了新的框架下，原有代码位于[phytium_pi_dev]([Files · phytium_pi_dev · 旋转轮椅 / proj2210132-基于飞腾派的Arceos移植与外设驱动开发 · GitLab (eduxiji.net)](https://gitlab.eduxiji.net/T202412799992620/project2210132-232991/-/tree/phytium_pi_dev?ref_type=heads))分支 | 代码编写完成，开始debug |
| ...至今     | 仍然在进行uvc摄像头的开发，进展良好                                                                                                                                                                                                                                     | ·              |

## 关于系统移植的工作

——多说无益，展示代码！

### 系统移植

由于Arceos从设计开始就以跨平台为目标，因此已经有了较为完善的启动[配置文件](./platforms/aarch64-phytium-pi.toml):

```toml
#./platforms/aarch64-phytium-pi.toml
# Architecture identifier.
arch = "aarch64"
# Platform identifier.
platform = "aarch64-phytium-pi"
# Platform family.
family = "aarch64-phytium-pi"

# Base address of the whole physical memory.
phys-memory-base = "0x8000_0000"
# Size of the whole physical memory.
phys-memory-size = "0x8000_0000" # 2G
# Base physical address of the kernel image.
kernel-base-paddr = "0x9010_0000"
# Base virtual address of the kernel image.
kernel-base-vaddr = "0xffff_0000_9010_0000"
# kernel-base-vaddr = "0x9010_0000"
# Linear mapping offset, for quick conversions between physical and virtual
# addresses.
 phys-virt-offset = "0xffff_0000_0000_0000"
#phys-virt-offset = "0x0000_0000_0000_0000"
# MMIO regions with format (`base_paddr`, `size`).
mmio-regions = [
  # ["0xFE00_B000", "0x1000"],      # mailbox
  # ["0xFE20_1000", "0x1000"],      # PL011 UART
  # ["0xFF84_1000", "0x8000"],      # GICv2    
  #["0x40000000", "0xfff_ffff"],      # pcie ecam


  # ["0x6_0000_0000", "0x4000_0000"] # pcie control


  ["0x2800_C000", "0x1000"],      # UART 0
  ["0x2800_D000", "0x1000"],      # UART 1
  ["0x2800_E000", "0x1000"],      # UART 2
  ["0x2800_F000", "0x1000"],      # UART 3
  # ["0x32a0_0000", "0x2_0000"],      # usb0
  # ["0x32a2_0000", "0x2_0000"],      # usb0
  # ["0x3200_C000", "0x2000"],      #Ethernet1
  # ["0x3200_E000", "0x2000"],      #Ethernet2
  # ["0x3080_0000", "0x8000"],      # GICv2    
  ["0x3000_0000","0x800_0000"],     #other devices
  ["0x4000_0000","0x4000_0000"],   # pcie 
  ["0x10_0000_0000", "0x20_0000_0000"], # pcie mmio 64

  # ["0x6_0000_0000", "0x6_3fff_ffff"] # pcie control
]
virtio-mmio-regions = []
# UART Address
uart-paddr = "0x2800_D000"
uart-irq = "24"

# GIC Address
gicc-paddr = "0xFF84_2000"
gicd-paddr = "0xFF84_1000"

# Base physical address of the PCIe ECAM space.
pci-ecam-base = "0x4000_0000"
# End PCI bus number.
pci-bus-end = "0x100"
# PCI device memory ranges.
pci-ranges = [
  ["0x58000000", "0x27ffffff"],   # pcie mmio 32
  ["0x10_0000_0000", "0x30_0000_0000"], # pcie mmio 64
  # ["0x5800_0000", "0x7fff_ffff"],

  # ["0x6_0000_0000", "0x6_3fff_ffff"]
]

# Size of the nocache memory region
nocache-memory-size = "0x70_0000"
```

此外还需要修改一些Makefile

```makefile
#Makefile
#...
ifeq ($(PLATFORM_NAME), aarch64-raspi4)
  include scripts/make/raspi4.mk
else ifeq ($(PLATFORM_NAME), aarch64-phytium-pi)
  include scripts/make/phytium-pi.mk #添加编译平台目标
else ifeq ($(PLATFORM_NAME), aarch64-bsta1000b)
  include scripts/make/bsta1000b-fada.mk
endif
#...
```

为了开发时的方便起见，我们还写了给自动上传编译结果至uboot以供引导的小脚本

```makefile
#scripts/make/phytium-pi.mk
#添加启动项
phytium: build #build出目标镜像并打包成uboot可用格式
 gzip -9 -cvf $(OUT_BIN) > arceos-phytium-pi.bin.gz
 mkimage -f tools/phytium-pi/phytium-pi.its arceos-phytiym-pi.itb
 @echo 'Built the FIT-uImage arceos-phytium-pi.itb'

chainboot: build #无需手动载入镜像，使用此启动项可以将编译结果自动加载进uboot
 tools/phytium-pi/yet_another_uboot_transfer.py /dev/ttyUSB0 115200 $(OUT_BIN)
 echo ' ' > minicom_output.log
 minicom -D /dev/ttyUSB0 -b 115200 -C minicom_output.log
```

```python
#tools/phytium-pi/yet_another_uboot_transfer.py
#!/usr/bin/env python3

#串口传输脚本

import sys
import time
import serial
from xmodem import XMODEM

def send_file(port, baudrate, file_path):
    # 打开串口
    ser = serial.Serial(port, baudrate, timeout=1)

    # 等待 U-Boot 提示符
    while True:
        line = ser.readline().decode('utf-8', errors='ignore').strip()
        print(line)
        if line.endswith('Phytium-Pi#'):
            break

    # 发送 loady 命令
    ser.write(b'loadx 0x90100000\n')
    time.sleep(0.5)

    # 等待 U-Boot 准备好接收文件
    while True:
        line = ser.readline().decode('utf-8', errors='ignore').strip()
        print(line)
        if 'Ready for binary' in line:
            break

    # 发送 'C' 字符开始传输
    ser.write(b'C')

    # 使用 xmodem 协议传输文件
    with open(file_path, 'rb') as f:
        def getc(size, timeout=1):
            return ser.read(size) or None

        def putc(data, timeout=1):
            return ser.write(data)

        modem = XMODEM(getc, putc)
        modem.send(f)

    # 关闭串口
    ser.close()

if __name__ == '__main__':
    if len(sys.argv) != 4:
        print("Usage: python script.py <port> <baudrate> <file_path>")
        sys.exit(1)

    port = sys.argv[1]
    baudrate = int(sys.argv[2])
    file_path = sys.argv[3]

    send_file(port, baudrate, file_path)
```

### 串口适配

代码详见该[crate文件夹](./crates/arm_pl011/src/pl011.rs)，其启动步骤大致为：

根据[手册](./doc/resources/飞腾派软件编程手册V1.0.pdf)定义需要的寄存器（默认波特率是115200，无需定义处理波特率相关寄存器）

```rust
register_structs! {
    /// Pl011 registers.
    Pl011UartRegs {
        /// Data Register.
        (0x00 => dr: ReadWrite<u32>),
        (0x04 => _reserved0),
        /// Flag Register.
        (0x18 => fr: ReadOnly<u32>),
        (0x1c => _reserved1),
        /// Control register.
        (0x30 => cr: ReadWrite<u32>),
        /// Interrupt FIFO Level Select Register.
        (0x34 => ifls: ReadWrite<u32>),
        /// Interrupt Mask Set Clear Register.
        (0x38 => imsc: ReadWrite<u32>),
        /// Raw Interrupt Status Register.
        (0x3c => ris: ReadOnly<u32>),
        /// Masked Interrupt Status Register.
        (0x40 => mis: ReadOnly<u32>),
        /// Interrupt Clear Register.
        (0x44 => icr: WriteOnly<u32>),
        (0x48 => @END),
    }
}
```

实现初始化，读写字符，响应中断

```rust
/// The Pl011 Uart
///
/// The Pl011 Uart provides a programing interface for:
/// 1. Construct a new Pl011 UART instance
/// 2. Initialize the Pl011 UART
/// 3. Read a char from the UART
/// 4. Write a char to the UART
/// 5. Handle a UART IRQ
pub struct Pl011Uart {
    base: NonNull<Pl011UartRegs>,
}

unsafe impl Send for Pl011Uart {}
unsafe impl Sync for Pl011Uart {}

impl Pl011Uart {
    /// Constrcut a new Pl011 UART instance from the base address.
    pub const fn new(base: *mut u8) -> Self {
        Self {
            base: NonNull::new(base).unwrap().cast(),
        }
    }

    const fn regs(&self) -> &Pl011UartRegs {
        unsafe { self.base.as_ref() }
    }

    /// Initializes the Pl011 UART.
    ///
    /// It clears all irqs, sets fifo trigger level, enables rx interrupt, enables receives
    pub fn init(&mut self) {
        // clear all irqs
        self.regs().icr.set(0x7ff);

        // set fifo trigger level
        self.regs().ifls.set(0); // 1/8 rxfifo, 1/8 txfifo.

        // enable rx interrupt
        self.regs().imsc.set(1 << 4); // rxim

        // enable receive
        self.regs().cr.set((1 << 0) | (1 << 8) | (1 << 9)); // tx enable, rx enable, uart enable
    }

    /// Output a char c to data register
    pub fn putchar(&mut self, c: u8) {
        while self.regs().fr.get() & (1 << 5) != 0 {}
        self.regs().dr.set(c as u32);
    }

    /// Return a byte if pl011 has received, or it will return `None`.
    pub fn getchar(&mut self) -> Option<u8> {
        if self.regs().fr.get() & (1 << 4) == 0 {
            Some(self.regs().dr.get() as u8)
        } else {
            None
        }
    }

    /// Return true if pl011 has received an interrupt
    pub fn is_receive_interrupt(&self) -> bool {
        let pending = self.regs().mis.get();
        pending & (1 << 4) != 0
    }

    /// Clear all interrupts
    pub fn ack_interrupts(&mut self) {
        self.regs().icr.set(0x7ff);
    }
}
```

### I2C移植

TODO

### USB驱动

usb驱动，在当前语境下，其全程为USB主机端驱动，是一套巨大的复合体系，其包含usb控制器驱动，usb系统驱动与usb驱动模块。

* USB控制器驱动

# USB驱动代码导读

## 系统架构

![structure](doc/figures/usb_main_architecture.png)

在这张图中，我们可以看到这些要点：

* 主机驱动层：usb系统发展至今，已经到了第四代了，目前市面上大多数usb主机控制器都是usb3.0主机控制器-xhci(eXtensible Host Controller Interface)，由于其可扩展性，因此usb4也在使用它，在可预见的将来，xhci还会存在相当长一段时间，但这并不意味着我们就可以忽略过去的几种主机控制器-ehci/ohci/uhci，甚至是在虚拟化的环境下-vhci，因此，对于不同的主机，抽象出一套统一的接口上有必要的。

* 驱动无关的设备抽象：usb协议的上层建筑，各种驱动千变万化，但是它们都有着共同的操作对象，因此我们在这里将所有驱动都会用到的设备数据抽象出来，进一步的说，其内包含：
  
  * 设备的描述符
    
    * 设备的描述符通过控制传输从设备获取，由厂家定义，描述符为树形结构，其拓扑一般如下：
      
      　　![descriptor-topology](https://chenlongos.com/Phytium-Car/assert/ch2-1_1.png)
  
  * 设备当前所使用的interface与config值与端点的概念
    
    * config-设备的配置-对应着上图中的配置描述符，有一个设备一次只能选择一个配置
    
    * interface-设备的功能，举个例子，无线鼠标接收器，在主机看来，其实往往是一个复合设备——他同时有鼠标/键盘/手柄的功能，每个功能都对应着一个接口描述符
    
    * endpoint-设备端点-对应着图中的端点描述符，“端点”在usb系统中，是主机与设备沟通的最小通道，端点本身也是一种状态机
      
      ![endpoint state machine](doc/figures/endpoint-state-machine.png)
      
      在初始状态下，除了永远保持开启的0号端点（控制端点），其余端点都处于未被配置的Disabled状态，当一个配置被选择时，主机需要根据这个配置下的所有端点描述符对对应的端点做初始化（确定端点的传输方式，传输间隔，传输大小等）。
  
  * 设备的slot id-当设备被插入时，主机就会为设备分配一个唯一的slot id，这么做的原因是物理接口往往是可扩展的，因此用物理接口号来识别设备会造成混淆

* USB驱动层：这一层则是对USB协议的封装，与主机控制器的内容并不相关，就像是其他的常见usb主机侧驱动一样（比如：linux，windows，circle os，redox...)它们都有一种叫做URB(USB Request Block)的结构体，其内记录了发起一次usb请求所需要的全部信息，这些信息在主机驱动层被解析为不同的主机控制器所对应的特定操作。而在我们的usb系统中，我们也设计了类似的结构：
  
  * URB:我们的系统中也具有URB的概念，但与其他的系统略有不同
    
    * 模版匹配：得益于rust强大的模版匹配机制，我们并不需要像其他系统一样设计一个庞大的结构体:
      
      ```rust
      //...
      #[derive(Clone)]
      pub struct URB<'a, O>
      where
          O: PlatformAbstractions,
      {
          pub device_slot_id: usize,
          pub operation: RequestedOperation<'a>,
          pub sender: Option<Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>>>,
      }
      //...
      #[derive(Debug, Clone)]
      pub enum RequestedOperation<'a> {
          Control(ControlTransfer),//<--------------|
          Bulk,//                  |
          Interrupt(InterruptTransfer),//      |
          Isoch,//                 |
          ConfigureDevice(Configuration<'a>),//   |
      }//                      |
      //...                     |
      #[derive(Debug, Clone)]   //        |
      pub struct ControlTransfer {// >---------------|
          pub request_type: bmRequestType,
          pub request: bRequest,
          pub index: u16,
          pub value: u16,
          pub data: Option<(usize, usize)>,
      }
      //...
      ```
      
      ```
      * 注意到RequestOperation这个枚举类型，其对应了usb的四种传输(控制，中断，块，同步)以及一种额外的请求：设备配置（如，为设备分配地址，硬重置设备端口等），在实际使用中，这种模式这给予了我们不同往日的灵活。
      ```
    
    * UCB：这个结构体包含了URB所发起的事件的完成信息（完成状态，是否有错误，回报信息），与urb形成了对称，当URB的时间完成时，UCB将会被创建，并通过URB中的“sender”字段所保存的请求发起者的引用，回报给请求发起者（即被驱动创建的设备实例）

* 驱动api：可以在图中看到，我们的驱动部分是可扩展的，甚至可以动态的加载驱动模块，有着非常巧妙的设计：
  
  * 首先，我们的驱动采用部分工厂模式+状态机的思想，其api定义如下：
    
    ```rust
    pub trait USBSystemDriverModule<'a, O>: Send + Sync
    where
        O: PlatformAbstractions,
    {
        fn should_active(
            &self,
            independent_dev: &DriverIndependentDeviceInstance<O>,
            config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
        ) -> Option<Vec<Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>>>>;
    
        fn preload_module(&self);
    }
    
    pub trait USBSystemDriverModuleInstance<'a, O>: Send + Sync
    where
        O: PlatformAbstractions,
    {
        fn prepare_for_drive(&mut self) -> Option<Vec<URB<'a, O>>>;
    
        fn gather_urb(&mut self) -> Option<URB<'a, O>>;
    
        fn receive_complete_event(&mut self, event: event::Allowed);
    }
    ```
    
    * USBSystemDriverModule：这是创建“驱动设备”的工厂，当有新设备被初始化完成时，usb层就会对每个驱动模块进行查询，当确定到有合适的驱动模块时，就会使对应的模块创建一个“驱动设备”的实例
    
    * USBSystemDriverModuleInstance：即上文所提到的“驱动设备”，该设备与下层的“驱动无关设备”并不耦合，具体的来说，驱动设备应当被设计为一种状态机，usb系统在开始运行后，就会在每个loop的通过轮询所有“驱动设备”的`gather_urb`方法收集URB请求（根据驱动设备的状态，可能得到不同的，甚至干脆就没有URB,这些都是根据驱动设备的当前状态及其内部实现来决定的，我们称为-tick），在获取到了一大堆的URB后，就会将他们统一提交给主机控制器层，并等待任务完成后将UCB提交回驱动设备以改变驱动设备的状态（我们称为-tock）
    
    * 也就是说，在正常运行的前提下，我们整个usb系统都是在不断的进行"tick-tock"，就像是时间轮一样：
      
      ![tick-tock-machine](/Users/dbydd/Documents/oscpmp/doc/figures/tick-tock-machine.png)

## 驱动案例：usb-hid鼠标驱动

## 代码结构

```log
.
├── abstractions
│   ├── dma.rs
│   └── mod.rs
├── err.rs
├── glue
│   ├── driver_independent_device_instance.rs
│   ├── glue_usb_host.rs
│   └── mod.rs
├── host
│   ├── data_structures
│   │   ├── host_controllers
│   │   │   ├── mod.rs
│   │   │   └── xhci
│   │   │       ├── context.rs
│   │   │       ├── event_ring.rs
│   │   │       ├── mod.rs
│   │   │       └── ring.rs
│   │   └── mod.rs
│   └── mod.rs
├── lib.rs
└── usb
    ├── descriptors
    │   ├── desc_configuration.rs
    │   ├── desc_device.rs
    │   ├── desc_endpoint.rs
    │   ├── desc_hid.rs
    │   ├── desc_interface.rs
    │   ├── desc_str.rs
    │   └── mod.rs
    ├── drivers
    │   ├── driverapi.rs
    │   └── mod.rs
    ├── mod.rs
    ├── operation
    │   └── mod.rs
    ├── trasnfer
    │   ├── control.rs
    │   ├── endpoints
    │   │   └── mod.rs
    │   ├── interrupt.rs
    │   └── mod.rs
    ├── universal_drivers
    │   ├── hid_drivers
    │   │   ├── hid_mouse.rs
    │   │   └── mod.rs
    │   └── mod.rs
    └── urb.rs
```
