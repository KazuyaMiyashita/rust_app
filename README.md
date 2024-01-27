
https://www.raspberrypi.com/documentation/microcontrollers/debug-probe.html

```
$ openocd -f interface/cmsis-dap.cfg -f target/rp2040.cfg -c "adapter speed 5000" -c "program target/thumbv6m-none-eabi/debug/rp2040-project-template verify reset exit"

$ openocd -f interface/cmsis-dap.cfg -f target/rp2040.cfg -c "adapter speed 5000"
```