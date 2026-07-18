use crate::cpu::AddressingMode::*;

const OPCODES_CYCLES: [u8; 256] = [
//  0   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
    7,  6,  0,  8,  3,  3,  5,  5,  3,  2,  2,  2,  4,  4,  6,  6, // 0
    2,  5,  0,  8,  4,  4,  6,  6,  2,  4,  2,  7,  4,  4,  7,  7, // 1
    6,  6,  0,  8,  3,  3,  5,  5,  4,  2,  2,  2,  4,  4,  6,  6, // 2
    2,  5,  0,  8,  4,  4,  6,  6,  2,  4,  2,  7,  4,  4,  7,  7, // 3
    6,  6,  0,  8,  3,  3,  5,  5,  3,  2,  2,  2,  3,  4,  6,  6, // 4
    2,  5,  0,  8,  4,  4,  6,  6,  2,  4,  2,  7,  4,  4,  7,  7, // 5
    6,  6,  0,  8,  3,  3,  5,  5,  4,  2,  2,  2,  5,  4,  6,  6, // 6
    2,  5,  0,  8,  4,  4,  6,  6,  2,  4,  2,  7,  4,  4,  7,  7, // 7
    2,  6,  2,  6,  3,  3,  3,  3,  2,  2,  2,  2,  4,  4,  4,  4, // 8
    2,  6,  0,  6,  4,  4,  4,  4,  2,  5,  2,  5,  5,  5,  5,  5, // 9
    2,  6,  2,  6,  3,  3,  3,  3,  2,  2,  2,  2,  4,  4,  4,  4, // A    
    2,  5,  0,  5,  4,  4,  4,  4,  2,  4,  2,  4,  4,  4,  4,  4, // B
    2,  6,  2,  8,  3,  3,  5,  5,  2,  2,  2,  2,  4,  4,  6,  6, // C
    2,  5,  0,  8,  4,  4,  6,  6,  2,  4,  2,  7,  4,  4,  7,  7, // D
    2,  6,  2,  8,  3,  3,  5,  5,  2,  2,  2,  2,  4,  4,  6,  6, // E
    2,  5,  0,  8,  4,  4,  6,  6,  2,  4,  2,  7,  4,  4,  7,  7, // F
];

#[derive(PartialEq, Clone, Copy)] 
pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Accumulator,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndirectX,
    IndirectY,
}

pub trait Bus {
    fn read(&mut self, address: u16) -> u8;
    fn write(&mut self, address: u16, data: u8);
}

pub struct Cpu {
    pub pc: u16, 
    pub sp: u8, 
    pub a: u8,   
    pub x: u8,   
    pub y: u8,   
    pub p: ParsedBites,   
    pub cycles: u64
}

#[derive(Debug)]
pub struct ParsedBites {
    pub carry: bool,
    pub zero: bool,
    pub interrupt: bool,
    pub decimal: bool,
    pub break_command: bool,
    pub unused: bool,
    pub overflow: bool,
    pub negative: bool,
    
}

impl ParsedBites {
    pub fn new() -> Self {
        ParsedBites { 
            carry:false, 
            zero: false, 
            interrupt: true, 
            decimal: false, 
            break_command: false, 
            unused: true, 
            overflow: false, 
            negative: false }
    }

    pub fn to_byte(&mut self) -> u8{
        let mut status_byte = 0u8;
        if self.carry { status_byte |= 0x01; }
        if self.zero { status_byte |= 0x02; }
        if self.interrupt { status_byte |= 0x04; }
        if self.decimal { status_byte |= 0x08; }
        if self.break_command { status_byte |= 0x10; }
        if self.unused { status_byte |= 0x20; }
        if self.overflow { status_byte |= 0x40; }
        if self.negative { status_byte |= 0x80; }
        status_byte
    }

    pub fn from_byte(&mut self, status_byte: u8) {
        self.carry = status_byte & 0x01 != 0;
        self.zero = status_byte & 0x02 != 0;
        self.interrupt = status_byte & 0x04 != 0;
        self.decimal = status_byte & 0x08 != 0;
        self.break_command = status_byte & 0x10 != 0;
        self.unused = status_byte & 0x20 != 0;
        self.overflow = status_byte & 0x40 != 0;
        self.negative = status_byte & 0x80 != 0;
    }
}

impl Cpu 
{
    pub fn new() -> Self {
        Self {
            pc: 0x0000, 
            sp: 0xFD,
            a: 0x00,
            x: 0x00,
            y: 0x00,
            p: ParsedBites::new(),
            cycles: 0,
        }
    }

    pub fn internal_reset(&mut self, bus: &mut impl Bus) {
            self.pc = ((bus.read(0xFFFD) as u16) << 8) | (bus.read(0xFFFC) as u16);
            self.sp = 0xFD;
            self.a = 0x00;
            self.x = 0x00;  
            self.y = 0x00;
            self.p = ParsedBites::new();
            self.cycles = 0;
        }

    pub fn nmi(&mut self, bus: &mut impl Bus) {
        self.push_u16(bus, self.pc);
        let status_byte = (self.p.to_byte() & !0x10) | 0x20; 
        self.push_u8(bus, status_byte);
        self.p.interrupt = true;
        self.pc = ((bus.read(0xFFFB) as u16) << 8) | (bus.read(0xFFFA) as u16);
        self.cycles += 7;
    }

    pub fn make_step_and_return_cycles(&mut self, bus: &mut impl Bus) -> u64{
        let cycle_old = self.cycles;
        let opcode = bus.read(self.pc);
        self. pc += 1;
        

        self.cycles += OPCODES_CYCLES[opcode as usize] as u64;
        match opcode {
            0x00 => self.brk(bus),
            0xEA => self.nop(),
            0x18 => self.clc(),
            0x38 => self.sec(),  
            0x58 => self.cli(),
            0x78 => self.sei(),
            0xB8 => self.clv(),
            0xD8 => self.cld(),
            0xF8 => self.sed(),  

            0x40 => self.rti(bus),

            0x10 => self.bpl(bus),
            0x30 => self.bmi(bus),
            0x50 => self.bvc(bus),
            0x70 => self.bvs(bus),
            0x90 => self.bcc(bus),
            0xB0 => self.bcs(bus),
            0xD0 => self.bne(bus),
            0xF0 => self.beq(bus),

            0x4C => self.jmp(bus, Absolute),
            0x6C => self.jmp(bus, Indirect),
            0x20 => self.jsr(bus, Absolute),
            0x60 => self.rts(bus),   

            0x48 => self.pha(bus),  
            0x08 => self.php(bus),  
            0x68 => self.pla(bus),  
            0x28 => self.plp(bus),  


            0xA9 => self.lda(bus, Immediate),
            0xA5 => self.lda(bus, ZeroPage),
            0xB5 => self.lda(bus, ZeroPageX),  
            0xAD => self.lda(bus, Absolute),
            0xBD => self.lda(bus, AbsoluteX),  
            0xB9 => self.lda(bus, AbsoluteY),  
            0xA1 => self.lda(bus, IndirectX),
            0xB1 => self.lda(bus, IndirectY),

            0xA2 => self.ldx(bus, Immediate),  
            0xA6 => self.ldx(bus, ZeroPage),   
            0xB6 => self.ldx(bus, ZeroPageY),  
            0xAE => self.ldx(bus, Absolute),   
            0xBE => self.ldx(bus, AbsoluteY),  

            0xA0 => self.ldy(bus, Immediate),
            0xA4 => self.ldy(bus, ZeroPage),
            0xB4 => self.ldy(bus, ZeroPageX),
            0xAC => self.ldy(bus, Absolute),
            0xBC => self.ldy(bus, AbsoluteX),

            0x85 => self.sta(bus, ZeroPage),
            0x95 => self.sta(bus, ZeroPageX),
            0x8D => self.sta(bus, Absolute),
            0x9D => self.sta(bus, AbsoluteX),
            0x99 => self.sta(bus, AbsoluteY),
            0x81 => self.sta(bus, IndirectX),
            0x91 => self.sta(bus, IndirectY),

            0x86 => self.stx(bus, ZeroPage),
            0x96 => self.stx(bus, ZeroPageY),
            0x8E => self.stx(bus, Absolute),

            0x84 => self.sty(bus, ZeroPage),
            0x94 => self.sty(bus, ZeroPageX),
            0x8C => self.sty(bus, Absolute),

            0xAA => self.tax(),
            0x8A => self.txa(),
            0xA8 => self.tay(),
            0x98 => self.tya(),
            0xBA => self.tsx(),
            0x9A => self.txs(),

            0x69 => self.adc(bus, Immediate),
            0x65 => self.adc(bus, ZeroPage),
            0x75 => self.adc(bus, ZeroPageX),
            0x6D => self.adc(bus, Absolute),
            0x7D => self.adc(bus, AbsoluteX),
            0x79 => self.adc(bus, AbsoluteY),
            0x61 => self.adc(bus, IndirectX),
            0x71 => self.adc(bus, IndirectY),

            0xE9 => self.sbc(bus, Immediate),  
            0xE5 => self.sbc(bus, ZeroPage),   
            0xF5 => self.sbc(bus, ZeroPageX),  
            0xED => self.sbc(bus, Absolute),   
            0xFD => self.sbc(bus, AbsoluteX),  
            0xF9 => self.sbc(bus, AbsoluteY),  
            0xE1 => self.sbc(bus, IndirectX),  
            0xF1 => self.sbc(bus, IndirectY),  

            0xC6 => self.dec(bus, ZeroPage),
            0xD6 => self.dec(bus, ZeroPageX),
            0xCE => self.dec(bus, Absolute),
            0xDE => self.dec(bus, AbsoluteX),

            0xE6 => self.inc(bus, ZeroPage),
            0xF6 => self.inc(bus, ZeroPageX),
            0xEE => self.inc(bus, Absolute),
            0xFE => self.inc(bus, AbsoluteX),

            0xE7 => self.isb(bus, ZeroPage),
            0xF7 => self.isb(bus, ZeroPageX),
            0xEF => self.isb(bus, Absolute),
            0xFF => self.isb(bus, AbsoluteX),
            0xFB => self.isb(bus, AbsoluteY),
            0xE3 => self.isb(bus, IndirectX),
            0xF3 => self.isb(bus, IndirectY),

            0x07 => self.slo(bus, ZeroPage),
            0x17 => self.slo(bus, ZeroPageX),
            0x0F => self.slo(bus, Absolute),
            0x1F => self.slo(bus, AbsoluteX),
            0x1B => self.slo(bus, AbsoluteY),
            0x03 => self.slo(bus, IndirectX),
            0x13 => self.slo(bus, IndirectY),

            0x67 => self.rra(bus, ZeroPage),
            0x77 => self.rra(bus, ZeroPageX),
            0x6F => self.rra(bus, Absolute),
            0x7F => self.rra(bus, AbsoluteX),
            0x7B => self.rra(bus, AbsoluteY),
            0x63 => self.rra(bus, IndirectX),
            0x73 => self.rra(bus, IndirectY),

            0x27 => self.rla(bus, ZeroPage),
            0x37 => self.rla(bus, ZeroPageX),
            0x2F => self.rla(bus, Absolute),
            0x3F => self.rla(bus, AbsoluteX),
            0x3B => self.rla(bus, AbsoluteY),
            0x23 => self.rla(bus, IndirectX),
            0x33 => self.rla(bus, IndirectY),

            0xCA => self.dex(),
            0x88 => self.dey(),
            0xE8 => self.inx(),
            0xC8 => self.iny(),

            0x29 => self.and(bus, Immediate),
            0x25 => self.and(bus, ZeroPage),
            0x35 => self.and(bus, ZeroPageX),
            0x2D => self.and(bus, Absolute),
            0x3D => self.and(bus, AbsoluteX),
            0x39 => self.and(bus, AbsoluteY),
            0x21 => self.and(bus, IndirectX),
            0x31 => self.and(bus, IndirectY),

            0x09 => self.ora(bus, Immediate),  
            0x05 => self.ora(bus, ZeroPage),   
            0x15 => self.ora(bus, ZeroPageX),  
            0x0D => self.ora(bus, Absolute),   
            0x1D => self.ora(bus, AbsoluteX),  
            0x19 => self.ora(bus, AbsoluteY),  
            0x01 => self.ora(bus, IndirectX),  
            0x11 => self.ora(bus, IndirectY),  

            0x49 => self.eor(bus, Immediate),  
            0x45 => self.eor(bus, ZeroPage),   
            0x55 => self.eor(bus, ZeroPageX),  
            0x4D => self.eor(bus, Absolute),   
            0x5D => self.eor(bus, AbsoluteX),  
            0x59 => self.eor(bus, AbsoluteY),  
            0x41 => self.eor(bus, IndirectX),  
            0x51 => self.eor(bus, IndirectY),  

            0xC9 => self.cmp(bus, Immediate),
            0xC5 => self.cmp(bus, ZeroPage),
            0xD5 => self.cmp(bus, ZeroPageX),
            0xCD => self.cmp(bus, Absolute),
            0xDD => self.cmp(bus, AbsoluteX),
            0xD9 => self.cmp(bus, AbsoluteY),
            0xC1 => self.cmp(bus, IndirectX),
            0xD1 => self.cmp(bus, IndirectY),

            0xE0 => self.cpx(bus, Immediate),
            0xE4 => self.cpx(bus, ZeroPage),
            0xEC => self.cpx(bus, Absolute),

            0xC0 => self.cpy(bus, Immediate),
            0xC4 => self.cpy(bus, ZeroPage),
            0xCC => self.cpy(bus, Absolute),

            0x0A => self.asl(bus, Accumulator),
            0x06 => self.asl(bus, ZeroPage),
            0x16 => self.asl(bus, ZeroPageX),
            0x0E => self.asl(bus, Absolute),
            0x1E => self.asl(bus, AbsoluteX),

            0x4A => self.lsr(bus, Accumulator),  
            0x46 => self.lsr(bus, ZeroPage),     
            0x56 => self.lsr(bus, ZeroPageX),    
            0x4E => self.lsr(bus, Absolute),     
            0x5E => self.lsr(bus, AbsoluteX),    

            0x2A => self.rol(bus, Accumulator),  
            0x26 => self.rol(bus, ZeroPage),     
            0x36 => self.rol(bus, ZeroPageX),    
            0x2E => self.rol(bus, Absolute),     
            0x3E => self.rol(bus, AbsoluteX),    

            0x6A => self.ror(bus, Accumulator),  
            0x66 => self.ror(bus, ZeroPage),     
            0x76 => self.ror(bus, ZeroPageX),    
            0x6E => self.ror(bus, Absolute),     
            0x7E => self.ror(bus, AbsoluteX),    

            0x24 => self.bit(bus, ZeroPage),
            0x2C => self.bit(bus, Absolute),

            _ => {println!("Чё за дерьмо: 0x{:02X}", opcode); self.nop();},
        }
        self.cycles - cycle_old
    }
        
    fn read_pc_u8(&mut self, bus: &mut impl Bus) -> u8 {
        let val = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        val
    }

    fn read_pc_u16(&mut self, bus: &mut impl Bus) -> u16 {
        let lo = bus.read(self.pc) as u16;
        let hi = bus.read(self.pc.wrapping_add(1)) as u16;
        self.pc = self.pc.wrapping_add(2);
        (hi << 8) | lo
    }

    fn pop_u8(&mut self, bus: &mut impl Bus) -> u8{
        self.sp  = self.sp.wrapping_add(1);
        bus.read(0x0100 + self.sp as u16)
    }

    fn push_u16(&mut self, bus: &mut impl Bus, value: u16) {
        // В 6502 стек растет вниз (от 0x01FF к 0x0100)
        bus.write(0x0100 + self.sp as u16, (value >> 8) as u8); 
        self.sp = self.sp.wrapping_sub(1);
        bus.write(0x0100 + self.sp as u16, (value & 0xFF) as u8); 
        self.sp = self.sp.wrapping_sub(1);
    }

    // Исправленный POP
    fn pop_u16(&mut self, bus: &mut impl Bus) -> u16 {
        self.sp = self.sp.wrapping_add(1);
        let low = bus.read(0x0100 + self.sp as u16) as u16;
        self.sp = self.sp.wrapping_add(1);
        let high = bus.read(0x0100 + self.sp as u16) as u16;
        (high << 8) | low
    }

    fn push_u8(&mut self, bus: &mut impl Bus, value: u8) {
        bus.write(0x0100 + self.sp as u16, value);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn page_cross_cycles(&mut self, address_1: u16, address_2: u16) -> u16 {
        let page_1 = address_1 >> 8;
        let page_2 = address_2 >> 8;
        if page_1 == page_2 {
            0
        }
        else {
            1
        }
    }   

    fn branch(&mut self, bus: &mut impl Bus, condition:bool) {
        let offset = self.read_pc_u8(bus);
        if condition {
            let signed_offset = offset as i8 as i16 as u16;
            self.pc = self.pc.wrapping_add(signed_offset);
        }
    }

    fn get_operand_address(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode, is_write: bool) -> u16 {
        match addressing_mode {
            AddressingMode::Immediate => {
                let addr = self.pc;
                self.pc += 1;
                addr
            }
            AddressingMode::ZeroPage => self.read_pc_u8(bus) as u16,
            AddressingMode::ZeroPageX => {
                let base = self.read_pc_u8(bus);
                base.wrapping_add(self.x) as u16
            }
            AddressingMode::ZeroPageY => {
                let base = self.read_pc_u8(bus);
                base.wrapping_add(self.y) as u16
            }
            AddressingMode::Absolute => self.read_pc_u16(bus),
            
            AddressingMode::AbsoluteX => {
                let base_address = self.read_pc_u16(bus);
                let target_address = base_address.wrapping_add(self.x as u16);
                
                if !is_write {
                    self.cycles += self.page_cross_cycles(base_address, target_address) as u64;
                }
                
                target_address
            }
            AddressingMode::AbsoluteY => {
                let base_address = self.read_pc_u16(bus);
                let target_address = base_address.wrapping_add(self.y as u16);
                
                if !is_write {
                    self.cycles += self.page_cross_cycles(base_address, target_address) as u64;
                }
                
                target_address
            }
            AddressingMode::IndirectY => {
                let base_zero_page = self.read_pc_u8(bus);
                let lo = bus.read(base_zero_page as u16) as u16;
                let hi = bus.read(base_zero_page.wrapping_add(1) as u16) as u16;
                let base_address = (hi << 8) | lo;
                let target_address = base_address.wrapping_add(self.y as u16);
                
                if !is_write {
                    self.cycles += self.page_cross_cycles(base_address, target_address) as u64;
                }
                
                target_address
            }
            AddressingMode::IndirectX => {
                let base_zero_page = self.read_pc_u8(bus);
                let ptr = base_zero_page.wrapping_add(self.x);
                let lo = bus.read(ptr as u16) as u16;
                let hi = bus.read(ptr.wrapping_add(1) as u16) as u16;
                (hi << 8) | lo
            }
            AddressingMode::Indirect => {
                let base_address = self.read_pc_u16(bus);
                let lo = bus.read(base_address) as u16;
                let hi = if (base_address & 0x00FF) == 0x00FF {
                    bus.read(base_address & 0xFF00) as u16
                } else {
                    bus.read(base_address.wrapping_add(1)) as u16
                };
                (hi << 8) | lo
            }
            _ => 0, 
        }
    }  
    
    fn beq(&mut self, bus: &mut impl Bus) {
        let cond = self.p.zero == true;
        self.branch(bus, cond);
    }

    fn bcs(&mut self, bus: &mut impl Bus) {
        let cond = self.p.carry == true;
        self.branch(bus, cond);
    }
        
    fn bcc(&mut self, bus: &mut impl Bus) {
        let cond = self.p.carry == false;
        self.branch(bus, cond);
    }

    fn bit(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let addr = self.get_operand_address(bus, addressing_mode, false);
        let value = bus.read(addr);
        self.p.zero = (self.a & value) == 0;
        self.p.negative = (value & 0x80) != 0;
        self.p.overflow = (value & 0x40) != 0; 
    }

    fn bmi(&mut self, bus: &mut impl Bus) {
        let cond = self.p.negative == true;
        self.branch(bus, cond);
    }

    fn bne(&mut self, bus: &mut impl Bus) {
        let cond = self.p.zero == false;
        self.branch(bus, cond);
    }
    
    fn bpl(&mut self, bus: &mut impl Bus) {
        let cond = self.p.negative == false;
        self.branch(bus, cond);
    }

    fn brk(&mut self, bus: &mut impl Bus) {
        self.pc = self.pc.wrapping_add(1);
        self.push_u16(bus, self.pc);
        let status_byte = self.p.to_byte() | 0x30;
        self.push_u8(bus, status_byte);
        self.p.interrupt = true;
        let lo = bus.read(0xFFFE) as u16;
        let hi = bus.read(0xFFFF) as u16;
        self.pc = (hi << 8) | lo;
        }

    fn bvc(&mut self, bus: &mut impl Bus) {
        let cond = self.p.overflow == false;
        self.branch(bus, cond);
    }

    fn bvs(&mut self, bus: &mut impl Bus) {
        let cond = self.p.overflow == true;
        self.branch(bus, cond);
    }

    fn clv(&mut self) {
        self.p.overflow = false
    }

    fn clc(&mut self) {
        self.p.carry = false
    }

    fn cld(&mut self) {
        self.p.decimal = false
    }

    fn cli(&mut self) {
        self.p.interrupt = false;
    }

    fn cmp(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let address = self.get_operand_address(bus, addressing_mode, false);
        let value = bus.read(address);     
        self.p.carry = self.a >= value;
        self.update_zero_and_negative_flags(self.a.wrapping_sub(value));
    }   

    fn cpx(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let address = self.get_operand_address(bus, addressing_mode, false);
        let value = bus.read(address);     
        self.p.carry = self.x >= value;
        self.update_zero_and_negative_flags(self.x.wrapping_sub(value));
    } 

    fn cpy(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let address = self.get_operand_address(bus, addressing_mode, false);
        let value = bus.read(address);     
        self.p.carry = self.y >= value;
        self.update_zero_and_negative_flags(self.y.wrapping_sub(value));
    } 

    fn dec(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let address = self.get_operand_address(bus, addressing_mode, true);
        let value = bus.read(address).wrapping_sub(1);     
        bus.write(address, value);
        self.update_zero_and_negative_flags(value);
    } 

    fn dex(&mut self) {
        self.x = self.x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.x);
    } 

    fn dey(&mut self) {
        self.y = self.y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.y);
    } 

    fn eor(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let addr = self.get_operand_address(bus, addressing_mode, false);
        let value = bus.read(addr);
        self.a = self.a ^ value;
        self.update_zero_and_negative_flags(self.a);
    }

    fn inc(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let address = self.get_operand_address(bus, addressing_mode, true);
        let value = bus.read(address).wrapping_add(1);     
        bus.write(address, value);
        self.update_zero_and_negative_flags(value);
    } 

    fn inx(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.x);
    } 

    fn iny(&mut self) {
        self.y = self.y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.y);
    } 

    fn isb(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let address = self.get_operand_address(bus, addressing_mode, false);
        let value = bus.read(address).wrapping_add(1);
        bus.write(address, value);

        let inverted_value = !value;
        let result = (self.a as u16) + (inverted_value as u16) + (self.p.carry as u16);
        let old_a = self.a;
        
        self.a = self.update_carry_and_return_u8_result(result);
        self.update_zero_and_negative_flags(self.a);
        self.p.overflow = ((old_a ^ self.a) & (inverted_value ^ self.a) & 0x80) != 0;
    }

    fn jmp(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let address = self.get_operand_address(bus, addressing_mode, false);
        self.pc = address;
    }

    fn jsr(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let address = self.get_operand_address(bus, addressing_mode, false);
        self.push_u16(bus, self.pc.wrapping_sub(1));
        self.pc = address;
    }

    fn lda(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let operand_address = self.get_operand_address(bus, addressing_mode, false);
        self.a = bus.read(operand_address);
        self.update_zero_and_negative_flags(self.a);
    }

    fn ldx(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let operand_address = self.get_operand_address(bus, addressing_mode, false);
        self.x = bus.read(operand_address);
        self.update_zero_and_negative_flags(self.x);
    }

    fn ldy(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let operand_address = self.get_operand_address(bus, addressing_mode, false);
        self.y = bus.read(operand_address);
        self.update_zero_and_negative_flags(self.y);
    }

    fn lsr(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        if addressing_mode == AddressingMode::Accumulator {
            self.p.carry = (self.a & 0x01) != 0;
            self.a >>= 1;
            self.update_zero_and_negative_flags(self.a);
        } else {
            let address = self.get_operand_address(bus, addressing_mode, true);
            let value = bus.read(address);
            self.p.carry = (value & 0x01) != 0;
            let result = value >> 1;
            self.update_zero_and_negative_flags(result);
            bus.write(address, result);
        }
    }

    fn nop(&mut self) {}  

    fn ora(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let addr = self.get_operand_address(bus, addressing_mode, false);
        let value = bus.read(addr);
        self.a = self.a | value;
        self.update_zero_and_negative_flags(self.a);
    }

    fn pha(&mut self, bus: &mut impl Bus) {
        self.push_u8(bus, self.a);
    }

    fn php(&mut self, bus: &mut impl Bus) {
        let status_byte = self.p.to_byte() | 0x30;
        self.push_u8(bus, status_byte);
    }

    fn pla(&mut self, bus: &mut impl Bus) {
        self.a = self.pop_u8(bus);
        self.update_zero_and_negative_flags(self.a);
    }

    fn plp(&mut self, bus: &mut impl Bus) {
        let status_byte = self.pop_u8(bus);
        self.p.from_byte(status_byte);
    }

    fn rol(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let old_carry = self.p.carry as u8;
        if addressing_mode == AddressingMode::Accumulator {
            self.p.carry = (self.a & 0x80) != 0;
            self.a = (self.a << 1) | old_carry;
            self.update_zero_and_negative_flags(self.a);
        } else {
            let address = self.get_operand_address(bus, addressing_mode, true);
            let value = bus.read(address);
            self.p.carry = (value & 0x80) != 0;
            let result = (value << 1) | old_carry;
            self.update_zero_and_negative_flags(result);
            bus.write(address, result);
        }
    }

    fn ror(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let old_carry = self.p.carry as u8;
        if addressing_mode == AddressingMode::Accumulator {
            self.p.carry = (self.a & 0x01) != 0;
            self.a = (self.a >> 1) | (old_carry << 7);
            self.update_zero_and_negative_flags(self.a);
        } else {
            let address = self.get_operand_address(bus, addressing_mode, true);
            let value = bus.read(address);
            self.p.carry = (value & 0x01) != 0;
            let result = (value >> 1) | (old_carry << 7);
            self.update_zero_and_negative_flags(result);
            bus.write(address, result);
        }
    }

    fn rra(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let address = self.get_operand_address(bus, addressing_mode, true);
        let value = bus.read(address);
            
        let result_u16: u16 = (value >> 1) as u16;

        let result = (self.a as u16) + (result_u16)  + (self.p.carry as u16);
        let old_a = self.a;
        self.a = self.update_carry_and_return_u8_result(result);
        self.update_zero_and_negative_flags(self.a);
        self.update_overflow(old_a, value, self.a);
    }

    fn rla(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let address = self.get_operand_address(bus, addressing_mode, true);
        let value = bus.read(address);
            
        let mut result = value << 1;

        result = self.a & result;
        self.a = self.update_carry_and_return_u8_result(result as u16);
        self.update_zero_and_negative_flags(self.a);
    }


    fn sbc(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let addr = self.get_operand_address(bus, addressing_mode, false);
        let value = bus.read(addr);
        
        let inverted_value = !value;
        
        let result = (self.a as u16) + (inverted_value as u16) + (self.p.carry as u16);
        let old_a = self.a;
        
        self.a = self.update_carry_and_return_u8_result(result);
        self.update_zero_and_negative_flags(self.a);
        
        self.p.overflow = ((old_a ^ self.a) & (inverted_value ^ self.a) & 0x80) != 0;
    }

    fn sec(&mut self) {
        self.p.carry = true;
    }

    fn sed(&mut self) {
        self.p.decimal = true;
    }

    fn sei(&mut self) {
        self.p.interrupt = true;
    }

    

    fn adc(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let addr = self.get_operand_address(bus, addressing_mode, false);
        let value = bus.read(addr);
        let result = (self.a as u16) + (value as u16)  + (self.p.carry as u16);
        let old_a = self.a;
        self.a = self.update_carry_and_return_u8_result(result);
        self.update_zero_and_negative_flags(self.a);
        self.update_overflow(old_a, value, self.a);
    }

    fn and(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let addr = self.get_operand_address(bus, addressing_mode, false);
        let value = bus.read(addr);
        self.a = self.a & value;
        self.update_zero_and_negative_flags(self.a);
    }

    fn asl(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        if addressing_mode == AddressingMode::Accumulator {
            let result: u16 = (self.a as u16) << 1;
            self.a = self.update_carry_and_return_u8_result(result);
            self.update_zero_and_negative_flags(self.a);
        }
        else {
            let address = self.get_operand_address(bus, addressing_mode, true);
            let value = bus.read(address);
            
            let result_u16: u16 = (value as u16) << 1;
            let result_u8 = self.update_carry_and_return_u8_result(result_u16);
            self.update_zero_and_negative_flags(result_u8);
            bus.write(address, result_u8);
        }
    }



    fn rti(&mut self, bus: &mut impl Bus) {
        let status_byte = self.pop_u8(bus);
        self.p.from_byte(status_byte);
        self.pc = self.pop_u16(bus);
    }

    fn rts(&mut self, bus: &mut impl Bus) {
        self.pc = self.pop_u16(bus).wrapping_add(1);
    }

    fn sta(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let address = self.get_operand_address(bus, addressing_mode, false);
        bus.write(address, self.a);
    }

    fn stx(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let address = self.get_operand_address(bus, addressing_mode, false);
        bus.write(address, self.x);
    }

    fn sty(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let address = self.get_operand_address(bus, addressing_mode, false);
        bus.write(address, self.y);
    }

    fn slo(&mut self, bus: &mut impl Bus, addressing_mode: AddressingMode) {
        let address = self.get_operand_address(bus, addressing_mode, true);
        let value = bus.read(address);
            
        let result_u16: u16 = (value << 1) as u16;
        let result_u8 = self.update_carry_and_return_u8_result(result_u16);
        self.update_zero_and_negative_flags(result_u8);
        bus.write(address, result_u8);              

        self.a = self.a | result_u8;
        self.update_zero_and_negative_flags(self.a);

    }

    fn tax(&mut self) {
        self.x = self.a;
        self.update_zero_and_negative_flags(self.x);
    }

    fn tay(&mut self) {
        self.y = self.a;
        self.update_zero_and_negative_flags(self.y);
    }

    fn tsx(&mut self) {
        self.x = self.sp;
        self.update_zero_and_negative_flags(self.x);
    }

    fn txs(&mut self) {
        self.sp = self.x;
    }

    fn tya(&mut self) {
        self.a = self.y;
        self.update_zero_and_negative_flags(self.a);
    }

    fn txa(&mut self) {
        self.a = self.x;
        self.update_zero_and_negative_flags(self.a);
    }


    fn update_zero_and_negative_flags(&mut self, result: u8) {
        self.p.zero = result == 0;
        self.p.negative = (result & 0x80) != 0;
    }

    fn update_carry_and_return_u8_result(&mut self, value: u16) -> u8{
        if value > 0xFF {
            self.p.carry = true;
            (value & 0x00FF) as u8
        }
        else {
            self.p.carry = false;
           value as u8
        }
    }

    fn update_overflow(&mut self, register_a: u8, value: u8, result: u8) {
        self.p.overflow = ((register_a ^ result) & (value ^ result) & 0x80) != 0;
    }
}



