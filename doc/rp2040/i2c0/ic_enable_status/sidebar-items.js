initSidebarItems({"enum":[["IC_EN_A","ic_en Status. This bit always reflects the value driven on the output port ic_en. - When read as 1, DW_apb_i2c is deemed to be in an enabled state. - When read as 0, DW_apb_i2c is deemed completely inactive. Note: The CPU can safely read this bit anytime. When this bit is read as 0, the CPU can safely read SLV_RX_DATA_LOST (bit 2) and SLV_DISABLED_WHILE_BUSY (bit 1).\\n\\n Reset value: 0x0"],["SLV_DISABLED_WHILE_BUSY_A","Slave Disabled While Busy (Transmit, Receive). This bit indicates if a potential or active Slave operation has been aborted due to the setting bit 0 of the IC_ENABLE register from 1 to 0. This bit is set when the CPU writes a 0 to the IC_ENABLE register while:\\n\\n (a) DW_apb_i2c is receiving the address byte of the Slave-Transmitter operation from a remote master;\\n\\n OR,\\n\\n (b) address and data bytes of the Slave-Receiver operation from a remote master.\\n\\n When read as 1, DW_apb_i2c is deemed to have forced a NACK during any part of an I2C transfer, irrespective of whether the I2C address matches the slave address set in DW_apb_i2c (IC_SAR register) OR if the transfer is completed before IC_ENABLE is set to 0 but has not taken effect.\\n\\n Note: If the remote I2C master terminates the transfer with a STOP condition before the DW_apb_i2c has a chance to NACK a transfer, and IC_ENABLE[0] has been set to 0, then this bit will also be set to 1.\\n\\n When read as 0, DW_apb_i2c is deemed to have been disabled when there is master activity, or when the I2C bus is idle.\\n\\n Note: The CPU can safely read this bit when IC_EN (bit 0) is read as 0.\\n\\n Reset value: 0x0"],["SLV_RX_DATA_LOST_A","Slave Received Data Lost. This bit indicates if a Slave-Receiver operation has been aborted with at least one data byte received from an I2C transfer due to the setting bit 0 of IC_ENABLE from 1 to 0. When read as 1, DW_apb_i2c is deemed to have been actively engaged in an aborted I2C transfer (with matching address) and the data phase of the I2C transfer has been entered, even though a data byte has been responded with a NACK.\\n\\n Note: If the remote I2C master terminates the transfer with a STOP condition before the DW_apb_i2c has a chance to NACK a transfer, and IC_ENABLE[0] has been set to 0, then this bit is also set to 1.\\n\\n When read as 0, DW_apb_i2c is deemed to have been disabled without being actively involved in the data phase of a Slave-Receiver transfer.\\n\\n Note: The CPU can safely read this bit when IC_EN (bit 0) is read as 0.\\n\\n Reset value: 0x0"]],"type":[["IC_EN_R","Reader of field `IC_EN`"],["R","Reader of register IC_ENABLE_STATUS"],["SLV_DISABLED_WHILE_BUSY_R","Reader of field `SLV_DISABLED_WHILE_BUSY`"],["SLV_RX_DATA_LOST_R","Reader of field `SLV_RX_DATA_LOST`"]]});