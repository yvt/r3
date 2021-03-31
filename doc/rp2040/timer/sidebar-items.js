initSidebarItems({"mod":[["alarm0","Arm alarm 0, and configure the time it will fire.\\n Once armed, the alarm fires when TIMER_ALARM0 == TIMELR.\\n The alarm will disarm itself once it fires, and can\\n be disarmed early using the ARMED status register."],["alarm1","Arm alarm 1, and configure the time it will fire.\\n Once armed, the alarm fires when TIMER_ALARM1 == TIMELR.\\n The alarm will disarm itself once it fires, and can\\n be disarmed early using the ARMED status register."],["alarm2","Arm alarm 2, and configure the time it will fire.\\n Once armed, the alarm fires when TIMER_ALARM2 == TIMELR.\\n The alarm will disarm itself once it fires, and can\\n be disarmed early using the ARMED status register."],["alarm3","Arm alarm 3, and configure the time it will fire.\\n Once armed, the alarm fires when TIMER_ALARM3 == TIMELR.\\n The alarm will disarm itself once it fires, and can\\n be disarmed early using the ARMED status register."],["armed","Indicates the armed/disarmed status of each alarm.\\n A write to the corresponding ALARMx register arms the alarm.\\n Alarms automatically disarm upon firing, but writing ones here\\n will disarm immediately without waiting to fire."],["dbgpause","Set bits high to enable pause when the corresponding debug ports are active"],["inte","Interrupt Enable"],["intf","Interrupt Force"],["intr","Raw Interrupts"],["ints","Interrupt status after masking & forcing"],["pause","Set high to pause the timer"],["timehr","Read from bits 63:32 of time\\n always read timelr before timehr"],["timehw","Write to bits 63:32 of time\\n always write timelw before timehw"],["timelr","Read from bits 31:0 of time"],["timelw","Write to bits 31:0 of time\\n writes do not get copied to time until timehw is written"],["timerawh","Raw read from bits 63:32 of time (no side effects)"],["timerawl","Raw read from bits 31:0 of time (no side effects)"]],"struct":[["RegisterBlock","Register block"]],"type":[["ALARM0","Arm alarm 0, and configure the time it will fire.\\n Once armed, the alarm fires when TIMER_ALARM0 == TIMELR.\\n The alarm will disarm itself once it fires, and can\\n be disarmed early using the ARMED status register."],["ALARM1","Arm alarm 1, and configure the time it will fire.\\n Once armed, the alarm fires when TIMER_ALARM1 == TIMELR.\\n The alarm will disarm itself once it fires, and can\\n be disarmed early using the ARMED status register."],["ALARM2","Arm alarm 2, and configure the time it will fire.\\n Once armed, the alarm fires when TIMER_ALARM2 == TIMELR.\\n The alarm will disarm itself once it fires, and can\\n be disarmed early using the ARMED status register."],["ALARM3","Arm alarm 3, and configure the time it will fire.\\n Once armed, the alarm fires when TIMER_ALARM3 == TIMELR.\\n The alarm will disarm itself once it fires, and can\\n be disarmed early using the ARMED status register."],["ARMED","Indicates the armed/disarmed status of each alarm.\\n A write to the corresponding ALARMx register arms the alarm.\\n Alarms automatically disarm upon firing, but writing ones here\\n will disarm immediately without waiting to fire."],["DBGPAUSE","Set bits high to enable pause when the corresponding debug ports are active"],["INTE","Interrupt Enable"],["INTF","Interrupt Force"],["INTR","Raw Interrupts"],["INTS","Interrupt status after masking & forcing"],["PAUSE","Set high to pause the timer"],["TIMEHR","Read from bits 63:32 of time\\n always read timelr before timehr"],["TIMEHW","Write to bits 63:32 of time\\n always write timelw before timehw"],["TIMELR","Read from bits 31:0 of time"],["TIMELW","Write to bits 31:0 of time\\n writes do not get copied to time until timehw is written"],["TIMERAWH","Raw read from bits 63:32 of time (no side effects)"],["TIMERAWL","Raw read from bits 31:0 of time (no side effects)"]]});