use std::fs::File;

type MapperReadFn = fn(&mut Cartridge, address: u16) -> u8;
type MapperWriteFn = fn(&mut Cartridge, address: u16, data: u8);

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum MirroringType {
    Horizontal,
    Vertical,
}

pub struct Cartridge {
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub mapper: u8,
    pub mapper_reader: MapperReadFn,
    pub mapper_writer: MapperWriteFn,
    pub mirroring: MirroringType,
}

impl Cartridge {
    pub fn choose_mapper(mapper_number: u8, prg_len: usize) -> (MapperReadFn, MapperWriteFn) {
        match mapper_number {
            0 => {
                if prg_len == 16384 {
                    (Cartridge::read_nrom_16kb, Cartridge::write_nrom)
                } else {
                    (Cartridge::read_nrom_32kb, Cartridge::write_nrom)
                }
            }
            _ => panic!("Invalid mapper"),
        }
    }

    fn read_nrom_16kb(&mut self, addr: u16) -> u8 {
        let index = ((addr - 0x8000) % 16384) as usize;
        self.prg_rom[index]
    }

    fn read_nrom_32kb(&mut self, addr: u16) -> u8 {
        let index = (addr - 0x8000) as usize;
        self.prg_rom[index]
    }

    fn write_nrom(&mut self, _address: u16, _data: u8) {
    }

    pub fn parse_rom(file_path: &str) -> Cartridge {
        let mut file = File::open(file_path).expect("Failed to open ROM file");
        let mut buffer = Vec::new();
        use std::io::Read;
        file.read_to_end(&mut buffer).expect("Failed to read ROM file");
        let (prg_size, chr_size) = Self::parse_header(&buffer);

        let lower_mapper = buffer[6] >> 4;
        let upper_mapper = buffer[7] & 0xF0;
        let mapper_number = upper_mapper | lower_mapper;

        let (reader, writer) = Self::choose_mapper(mapper_number, prg_size);

        Cartridge {
            prg_rom: Self::parse_prg_rom(&buffer, prg_size),
            chr_rom: Self::parse_chr_rom(&buffer, prg_size, chr_size),
            mapper: mapper_number,
            mirroring: match buffer[6] & 0x1 {
                0 => MirroringType::Horizontal,
                1 => MirroringType::Vertical,
                _ => panic!("Invalid mirroring bit"),
            },
            mapper_reader: reader,
            mapper_writer: writer,
        }
    }

    pub fn parse_header(rom_data: &[u8]) -> (usize, usize) {
        let prg_size = rom_data[4] as usize * 16 * 1024;
        let chr_size = rom_data[5] as usize * 8 * 1024;
        (prg_size, chr_size)
    }

    pub fn parse_prg_rom(rom_data: &[u8], prg_size: usize) -> Vec<u8> {
        rom_data[16..16 + prg_size].to_vec()
    }

    pub fn parse_chr_rom(rom_data: &[u8], prg_size: usize, chr_size: usize) -> Vec<u8> {
        rom_data[16 + prg_size..16 + prg_size + chr_size].to_vec()
    }
}