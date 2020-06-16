#! /bin/bash

arm-none-eabi-objcopy -O binary $1 $1.bin
st-flash write $1.bin  0x8000000
rm $1.bin
openocd -f interface/stlink-v2.cfg -f target/stm32f1x.cfg -c "init; reset; arm semihosting enable;"
#sudo openocd -f interface/stlink-v2.cfg -f target/stm32f1x.cfg -c "program $1 verify reset exit"
