# USB转串口测试流程
参考[hid测试]（https://github.com/Jasonhonghh/arceos_experiment/blob/usb-serial-dev/doc/apps_usb-hid.md）通过USB转串口把内核文件加载到飞腾派中运行即可。
## 物理连接
1. CH340(1)连接飞腾派和主机，用于把内核文件加载进入飞腾派和命令交互。
2. CH340(2)的USB端接飞腾派的USB口，另外一端接串口设备，蓝色外侧的USB口。
3. CH340(3)用于接受CH340(2)转换的串口数据，USB端接入主机。（CH340(3)仅仅用于接受和发送串口数据）
## 软件启动
可能需要修改`scripts/make/phytium-pi.mk`中的串口参数，如果使用的是厂商版的USB转串口驱动，串口名称是`ttyCH341USB0`。如果使用的是Linux自带的驱动，串口名称为`ttyUSB0`。

其他的启动过程请参考[hid测试]（https://github.com/Jasonhonghh/arceos_experiment/blob/usb-serial-dev/doc/apps_usb-hid.md），在项目根目录下执行`make A=apps/usb-hid PLATFORM=aarch64-phytium-pi LOG=trace chainboot`会自动加载USB驱动。
## 效果
我在测试时，使用CH340(3)来接受CH340(2)发送的数据，和向CH340(2)发送一些串口数据，使用minicom可以查看通信过程，**暂时要把波特率调整成9600,其他参数默认即可。**
### 向串口写数据
目前驱动向串口写数据的接口还没修改完，为了测试写入数据的功能，驱动中直接封装了向串口写入"hello,world"的代码，所以在自动加载USB驱动后，CH340(2)的串口端，会收到"hello,world"，可以通过minicom查看。
### 从串口读数据
读取的串口数据会显示在日志当中。从minicom写入“hello,phytiumpi”,可以在Arceos日志中看到accepted data:“hello,phytiumpi”。

注：传输的数据实际是解码后打印出来的。

