# ChangeLog

## 2025-1-24 v 0.0.0 usb-serial-dev

### Added

### Changed

### Removed

### Fixed

### Question

## 2025-1-24 v 0.0.0 usb-serial-dev

### Added
- `crates/driver_usb/src/usb/universal_drivers/cdc_drivers/cdc_serial.rs`中添加`CdcSerialDriver`的`prepare_for_drive`方法，构造一个设置配置描述符的URB请求。



### Changed
- `crates/driver_usb/src/usb/universal_drivers/cdc_drivers/cdc_serial.rs`的驱动实例增加了插槽号，接口值、配置值等字段，便于配置URB。
- `crates/driver_usb/src/usb/universal_drivers/cdc_drivers/cdc_serial.rs`修改了驱动实例的new方法，修改为new_and_init方法，添加内容。
- `crates/driver_usb/src/usb/universal_drivers/cdc_drivers/cdc_serial.rs`修改should_active中返回值的内容。

### Removed

### Fixed

### Question


## 2025-1-23 v 0.0.0usb-serial-dev

### Added
- CH340描述符信息，分别通过Linux和driver_usb的方法读到。doc中添加“ch340信息”（ch340描述符）。

### Changed
- CH340的USB设备类型是255,不是CDC，因此需要修改`crates/driver_usb/src/usb/descriptors/parser.rs`的`determine`函数逻辑。切换为VendorSpecific。

### Removed

### Fixed

### Question

## 2025-1-22  v 0.0.0 usb-serial-dev

### Added

- `src\usb\descriptors\parser.rs`中`ParserMetaData`枚举添加`USBToSerial`，用于标识USB转串口设备。
- `src\usb\descriptors\parser.rs`中`ParserMetaData`枚举的`determine`方法，添加`StandardUSBDeviceClassCode::CommunicationsAndCDCControl`设备类型的定义，返回`USBToSerial`。
- `src\usb\universal_drivers\cdc_drivers\cdc_serial.rs`的` CdcSerialDriver`结构体添加`config`成员，`Arc`和`SpinNoIrq`封装的`USBSystemConfig<O>`类型。
- `src\usb\universal_drivers\cdc_drivers\cdc_serial.rs`的` CdcSerialDriver`结构体添加`new`方法。
- `src\usb\universal_drivers\cdc_drivers\cdc_serial.rs`中补充`CdcSerialDriverModule`的`should_active`的具体实现。获取设备类型，若为`cdc`，则需要启用。

### Changed

### Removed

### Fixed

### Question

- USB转串口应该属于`StandardUSBDeviceClassCode::CommunicationsAndCDCControl`。目前整个系统只会使用一种USB转串口设备，所以暂时仅使用设备类别来匹配驱动。
- uvc驱动使用`ParserMetaData`来匹配驱动模块（should_active），而hid_mouse使用设备描述符中的class来表示。**添加了USB转串口的`ParserMetaData`的定义，但是最后USB转串口的驱动模块中还是使用设备描述符中的class来匹配驱动。**
- 


## 2025-1-16   v 0.0.0 usb-serial-dev

### Added

- 签出一个新的usb-serial-dev分支，用于修改代码，usb-camera-base分支用于与主仓库同步，文档都编写在usb-camera-base分支的doc中。
- `crates\driver_usb\src\usb\universal_drivers`中新建cdc_drivers目录。其中新建cdc_serial.rs和mod.rs。用于开发usb转串口的主要代码。
- cdc__serial.rs中新建“驱动模块”`CdcSerialDriverModule`和“驱动设备”`CdcSerialDriver`。
- cdc_serial.rs中为“驱动设备”`CdcSerialDriver`中实现`USBSystemDriverModuleInstance`特征的三个方法，未实现具体代码。
- cdc_serial.rs中为“驱动模块”`CdcSerialDriverModule`实现`USBSystemDriverModule`特征的两个方法，未实现具体代码。
- mod.rs中声明cdc_serial模块。
- doc目录中添加ChangeLog.md，即本文件。
- `crates\driver_usb\src\usb\mod.rs`中`USBDriverSystem`的init方法中添加对`USBSystemDriverModule`驱动模块的加载。

### Changed

### Removed

### Fixed

### Question

- `cdc_serial.rs`引入`descriptors::{desc_device::StandardUSBDeviceClassCode, desc_endpoint::Endpoint}`，这个描述符需要修改吗。
