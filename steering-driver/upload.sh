#! /bin/bash

arm-none-eabi-objcopy -O binary $1 $1.bin
st-flash write $1.bin  0x8000000
rm $1.bin
#sudo openocd -f interface/stlink-v2.cfg -f target/stm32f1x.cfg -c "program $1 verify reset exit"
