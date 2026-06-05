use crate::ppu;
use crate::cartridge;
use crate::joypad;
use crate::cpu::Bus;

pub struct Ram {
    pub ram: Vec<u8>,
}

impl Ram {
    pub fn new() -> Self {
        Self {
            ram: vec![0; 0x0800],
        }
    }
}

pub struct Console {
    pub ram: Ram,
    pub ppu: ppu::Ppu,
    pub cartridge: cartridge::Cartridge,
    pub joypad: joypad::Joypad,
}

impl Bus for Console {
    fn read(&mut self, address: u16) -> u8 {
        match address {
            0x0000..=0x1FFF => {
                self.ram.ram[(address % 0x0800) as usize]
            }
            0x2000..=0x3FFF => {
                self.ppu.read_register(address % 8)
            }
            0x4016 => {self.joypad.read_state()}
            0x4020..=0xFFFF => { (self.cartridge.mapper_reader)(&mut self.cartridge, address) }
            _ => 0,
        }
    }
    fn write(&mut self, address: u16, data: u8) {

        match address {
            0x0000..=0x1FFF => {
                self.ram.ram[(address % 0x0800) as usize] = data;
            }
            0x2000..=0x3FFF => {
                self.ppu.write_register(address % 8, data);
            }
            0x4014 => {
                let page_address = (data as u16) << 8;
                for i in 0..256 {
                    let byte = self.ram.ram[((page_address + i) % 0x0800) as usize];
                    self.ppu.oam[i as usize] = byte;
                    }
                }
            0x4016 => {self.joypad.step(data);}
            0x4020..=0xFFFF => { (self.cartridge.mapper_writer)(&mut self.cartridge, address, data); }
          
            _ => {}
        }
    }

}