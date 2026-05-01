Source: https://www.infinite-loop.at/Power20/Documentation/Power20-ReadMe/AD-Custom_Chips.html

VIA1: 0x9110
VIA2: 0x9120

91x0: Port B Data Register
91x1: Port A Data Register
91x2: Port B Data Direction Register (0: Input ~ 1: Output)
91x3: Port A Data Direction Register (0: Input ~ 1: Output)
91x4: Counter 1: Counter Register Bits 7…0
       Read:  Current State of Counters 1
       Write: Value to be loaded into Counter 1 at the next restart.
91x5: Counter 1: Counter Register Bits 15…8
       Read:  Current State of Counters 1
       Write: Value to be loaded into Counter 1 at the next restart.
91x6: Counter 1: Latch Register - Bits 7…0
       Value to be loaded into Counter 1 at the next restart.
91x7: Counter 1: Latch Register - Bits 15…8
       Value to be loaded into Counter 1 at the next restart.
91x8: Counter 2 Counter Register - Bits 7…0
       Read:  Current State of Counters 2
       Write: Value to be loaded into Counter 2 at the next restart.
91x9: Counter 2 Counter Register - Bits 15…8
       Read:  Current State of Counters 2
       Write: Value to be loaded into Counter 2 at the next restart
       and immediate restart.
91xA: Serial Data Register
91xB: Auxiliary Control Register
       Bit 7: Underflow of Counter 1 is indicated at Pin PB7
       Bit 6: 0: Automatically restart Timer 1 after Countdown has
                 completed
              1: Timer 1 is stopped when Countdown is complete.
       Bit 5: Counter 2 Trigger Source (1: System Clock - 0:PB6-Pin)
       Bit 4…2: Control Bits for the Serial Data Register
       Bit 1: Port B Latch Enable
       Bit 0: Port A Latch Enable
91xC: Periphery Control Register
       Bits 7…5: CB2 Handshake Control
            000 ~ Input: Interr. on negative Edge
                  Read or Write access to Port B clears the Interr.
            001 ~ Input: Interr. on negative Edge
                  Read or Write access to Port B does not clear
                  the Interr.
            010 ~ Input: Interr. on positive Edge
                  Read or Write access to Port B clears the Interr.
            011 ~ Input: Interr. on positive Edge
                  Read or Write access to Port B does not clear
                  the Interr.
            100 ~ Output: Write to Port B Data Reg. sets CB2 to 0
                          Change of CB1 sets CB2 to 1
            100 ~ Output: Write to Port B Data Reg. sets CB2 to 0
                          for one clockcycle
            110 ~ Output Low
            111 ~ Output High
       Bit 4: CB1 Input: 0: Interr. on negative Edge ~ 1: pos. Edge
       Bits 3…1: CA2 Handshake Control
            Like Bits 7…5 but for Port A Data Reg.
       Bit 0: CA1 Input: Like Bit 4
       There is no way to detect the current state of Port C. Only
       changes can be detected using interrupts.
91xD: Interrupt Flag Register
       Bit 7: Control Bit
       Bit 6: Timer 1 Underflow
       Bit 5: Timer 2 Underflow
       Bit 4: CB1 Interrupt
       Bit 3: CB2 Interrupt
       Bit 2: Serial Data Transmission completed
       Bit 1: CA1 Handshake Interrupt
       Bit 0: CA2 Handshake Interrupt
91xE: Interrupt Control Register
       Bitwise correspondence to 91xD.
91xF: Port A Data Register (like 91x1, but no Handshake)

9110: VIA1 Port B
       Bit 7: RS232: DSR (Data Set Ready)
       Bit 6: RS232: CTS (Clear to Send)
       Bit 5: Unused
       Bit 4: RS232: CF  (Receive Line Signal)
       Bit 3: RS232: RI  (Ring Indicator)
       Bit 2: RS232: DTR (Data Terminal Ready)
       Bit 1: RS232: RTS (Request to Send)
       Bit 0: RS232: RXD (Receive Data)

911C: VIA1 Port C
       CB2: RS232: SOut (Transmitted Data)
       CB1: RS232: SIn  (Interrupt for SIn)

9111: VIA1 Port A
       Bit 7: IEC-Bus: ATN Output
       Bit 6: Tape Button Query
       Bit 5: Joystick Fire
       Bit 4: Joystick Left
       Bit 3: Joystick Down
       Bit 2: Joystick Up
       Bit 1: Unused
       Bit 0: Unused

9120: VIA2 Port B
       Bits 7…0: Keyboard Row Selection
       Bit 7: Joystick Right
       Bit 3: Tape Write Data Line

9121: VIA2 Port A
       Bits 7…0: Keyboard Column Result

912C: VIA2 Port C
       CB2: IEC-BUS: Data Output
       CB1: IEC-BUS: Data Input
       CA2: IEC-BUS: Clock Output
       CA1: Tape Read Data Line