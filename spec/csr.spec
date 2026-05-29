# Unprivileged Floating-Point CSRs
0x001,fflags => fcsr
0x002,frm => fcsr
0x003,fcsr
# # Machine Information Registers
# 0xF11,M,RO,mvendorid
# 0xF12,M,RO,marchid
# 0xF13,M,RO,mimpid
# 0xF14,M,RO,mhartid
# 0xF15,M,RO,mconfigptr
# Machine Trap Setup
# 0x300,M,RW,mstatus
0x301,misa
# 0x302,M,RW,medeleg
# 0x303,M,RW,mideleg
# 0x304,M,RW,mie
# 0x305,M,RW,mtvec
# 0x306,M,RW,mcounteren
# # Machine Trap Handling
# 0x340,M,RW,mscratch
# 0x341,M,RW,mepc
# 0x342,M,RW,mcause
# 0x343,M,RW,mtval
# 0x344,M,RW,mip
# 0x34A,M,RW,mtinst
# 0x34B,M,RW,mtval2
# # Machine Configuration
# 0x30A,M,RW,menvcfg
# # Machine Counter/Timers
# 0xB00,M,RW,mcycle
# 0xB02,M,RW,minstret
# 0xB03,M,RW,mhpmcounter3
# 0xB04,M,RW,mhpmcounter4
# 0xB05,M,RW,mhpmcounter5
# 0xB06,M,RW,mhpmcounter6
# 0xB07,M,RW,mhpmcounter7
# 0xB08,M,RW,mhpmcounter8
# 0xB09,M,RW,mhpmcounter9
# 0xB0A,M,RW,mhpmcounter10
# 0xB0B,M,RW,mhpmcounter11
# 0xB0C,M,RW,mhpmcounter12
# 0xB0D,M,RW,mhpmcounter13
# 0xB0E,M,RW,mhpmcounter14
# 0xB0F,M,RW,mhpmcounter15
# 0xB10,M,RW,mhpmcounter16
# 0xB11,M,RW,mhpmcounter17
# 0xB12,M,RW,mhpmcounter18
# 0xB13,M,RW,mhpmcounter19
# 0xB14,M,RW,mhpmcounter20
# 0xB15,M,RW,mhpmcounter21
# 0xB16,M,RW,mhpmcounter22
# 0xB17,M,RW,mhpmcounter23
# 0xB18,M,RW,mhpmcounter24
# 0xB19,M,RW,mhpmcounter25
# 0xB1A,M,RW,mhpmcounter26
# 0xB1B,M,RW,mhpmcounter27
# 0xB1C,M,RW,mhpmcounter28
# 0xB1D,M,RW,mhpmcounter29
# 0xB1E,M,RW,mhpmcounter30
# 0xB1F,M,RW,mhpmcounter31
# # Machine Counter Setup
# 0x320,M,RW,mcountinhibit
# 0x323,M,RW,mhpmevent3
# 0x324,M,RW,mhpmevent4
# 0x325,M,RW,mhpmevent5
# 0x326,M,RW,mhpmevent6
# 0x327,M,RW,mhpmevent7
# 0x328,M,RW,mhpmevent8
# 0x329,M,RW,mhpmevent9
# 0x32A,M,RW,mhpmevent10
# 0x32B,M,RW,mhpmevent11
# 0x32C,M,RW,mhpmevent12
# 0x32D,M,RW,mhpmevent13
# 0x32E,M,RW,mhpmevent14
# 0x32F,M,RW,mhpmevent15
# 0x330,M,RW,mhpmevent16
# 0x331,M,RW,mhpmevent17
# 0x332,M,RW,mhpmevent18
# 0x333,M,RW,mhpmevent19
# 0x334,M,RW,mhpmevent20
# 0x335,M,RW,mhpmevent21
# 0x336,M,RW,mhpmevent22
# 0x337,M,RW,mhpmevent23
# 0x338,M,RW,mhpmevent24
# 0x339,M,RW,mhpmevent25
# 0x33A,M,RW,mhpmevent26
# 0x33B,M,RW,mhpmevent27
# 0x33C,M,RW,mhpmevent28
# 0x33D,M,RW,mhpmevent29
# 0x33E,M,RW,mhpmevent30
# 0x33F,M,RW,mhpmevent31