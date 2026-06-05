use crate::cartridge::Cartridge;
use crate::cartridge::MirroringType; 

pub struct Ppu {
    pub screen_buffer: Vec<u8>,
    pub nmi_interrupt: bool,

    chr: Vec<u8>,
    mirroring: MirroringType,
    vram: [u8; 2048],
    palette: [u8; 32],
    pub oam: [u8; 256],

    ctrl: u8,
    mask: u8,
    status: u8,
    oam_addr: u8,

    v: u16, 
    t: u16, 
    x: u8,  
    w: bool,

    read_buffer: u8,
    scanline: i16,
    cycle: u16,

    ppu_data_buffer: u8,

    bg_shift_pattern_low: u16,
    bg_shift_pattern_high: u16,
    bg_shift_attrib_low: u16,
    bg_shift_attrib_high: u16,

    bg_next_tile_id: u8,
    bg_next_attribute: u8,
    bg_next_pattern_low: u8,
    bg_next_pattern_high: u8,

    sprite_count: usize,
    sprite_patterns_low: [u8; 8],
    sprite_patterns_high: [u8; 8],
    sprite_attributes: [u8; 8],
    sprite_positions_x: [u8; 8],
    sprite_is_zero: [bool; 8],

    odd_frame: bool,
}

impl Ppu {
    pub fn new(cartridge: &Cartridge) -> Self {
        Self {
            sprite_count: 0,
            sprite_patterns_low: [0; 8],
            sprite_patterns_high: [0; 8],
            sprite_attributes: [0; 8],
            sprite_positions_x: [0; 8],
            sprite_is_zero: [false; 8],
            screen_buffer: vec![0; 256 * 240],
            nmi_interrupt: false,
            chr: cartridge.chr_rom.clone(),
            mirroring: cartridge.mirroring,
            vram: [0; 2048],
            palette: [0; 32],
            oam: [0; 256],
            ctrl: 0,
            mask: 0,
            status: 0,
            oam_addr: 0,
            v: 0,
            t: 0,
            x: 0,
            w: false,
            read_buffer: 0,
            scanline: 0,
            cycle: 0,
            ppu_data_buffer: 0,
            bg_shift_pattern_low: 0,
            bg_shift_pattern_high: 0,
            bg_shift_attrib_low: 0,
            bg_shift_attrib_high: 0,
            bg_next_tile_id: 0,
            bg_next_attribute: 0,
            bg_next_pattern_low: 0,
            bg_next_pattern_high: 0,
            odd_frame: false,
        }
    }

    pub fn read_register(&mut self, reg: u16) -> u8 {
        let mapped_reg = reg % 8;
        let res = match mapped_reg {
            2 => self.read_ppu_status(),
            4 => self.read_oam_data(),
            7 => self.read_ppu_data(),
            _ => self.ppu_data_buffer,
        };

        if mapped_reg == 2 {
            self.ppu_data_buffer = (res & 0xE0) | (self.ppu_data_buffer & 0x1F);
        } else {
            self.ppu_data_buffer = res;
        }
        res
    }

    pub fn write_register(&mut self, reg: u16, data: u8) {
        self.ppu_data_buffer = data;
        match reg % 8 {
            0 => self.write_ppu_ctrl(data),
            1 => self.write_ppu_mask(data),
            2 => {},
            3 => self.write_oam_addr(data),
            4 => self.write_oam_data(data),
            5 => self.write_ppu_scroll(data),
            6 => self.write_ppu_addr(data),
            7 => self.write_ppu_data(data),
            _ => unreachable!(),
        }
    }

    pub fn step(&mut self) {
        // Корректный пропуск цикла на нечетном кадре без двойного инкремента
        if self.scanline == -1 && self.cycle == 0 && self.odd_frame && (self.mask & 0x18) != 0 {
            self.cycle = 1;
        } else {
            self.cycle += 1;
            if self.cycle >= 341 {
                self.cycle = 0;
                self.scanline = self.scanline.wrapping_add(1);
                if self.scanline > 261 {
                    self.scanline = -1;
                    self.odd_frame = !self.odd_frame;
                }
            }
        }

        // Установка флага VBlank на 241-й строке
        if self.scanline == 241 && self.cycle == 1 {
            self.status |= 0x80;
            if (self.ctrl & 0x80) != 0 {
                self.nmi_interrupt = true;
            }
        }

        // Очистка флагов рендеринга на пререндер-строке
        if self.scanline == -1 && self.cycle == 1 {
            self.status &= !0xE0;
        }

        let rendering_enabled = (self.mask & 0x18) != 0;

        match self.scanline {
            -1..=239 => {
                // Выборка и сдвиги фона происходят как при активном рендере, так и при префетче (321..336)
                if (1..=256).contains(&self.cycle) || (321..=336).contains(&self.cycle) {
                    self.fetch_bg_data();
                    
                    if (1..=256).contains(&self.cycle) {
                        if self.scanline != -1 {
                            self.render_pixel();
                        }
                    }

                    if rendering_enabled {
                        // Сдвигаем регистры на каждом активном такте
                        self.shift_bg_registers();
                        
                        // Загружаем данные строго ПОСЛЕ того, как завершился 8-й сдвиг текущего тайла!
                        if self.cycle % 8 == 0 {
                            self.load_bg_shift_registers();
                        }
                    }
                }

                if rendering_enabled {
                    // Копирование горизонтального скролла на 257-м цикле
                    if self.cycle == 257 {
                        self.v = (self.v & !0x041F) | (self.t & 0x041F);
                        if self.scanline != -1 {
                            self.evalute_sprite_for_overflow();
                            self.fetch_sprites();
                        }
                    }

                    // Копирование вертикального скролла в конце пререндера
                    if self.scanline == -1 && (280..=304).contains(&self.cycle) {
                        self.v = (self.v & !0x7BE0) | (self.t & 0x7BE0);
                    }
                }
            }
            _ => {}
        }
    }

    fn render_pixel(&mut self) {
        let x = (self.cycle - 1) as usize;
        let y = self.scanline as usize;

        if x >= 256 || y >= 240 {
            return;
        }

        if (self.mask & 0x18) == 0 {
            let bg_color = if self.v >= 0x3F00 && self.v <= 0x3FFF {
                self.read_ppu_address(self.v)
            } else {
                self.read_ppu_address(0x3F00)
            };
            self.screen_buffer[y * 256 + x] = bg_color;
            return;
        }

        let (bg_index, bg_palette) = self.get_bg_pixel();
        let (spr_index, spr_palette, spr_priority, is_zero) = self.get_sprite_pixel();

        let final_color_byte;

        if bg_index == 0 && spr_index == 0 {
            final_color_byte = self.read_ppu_address(0x3F00);
        } else if bg_index == 0 && spr_index != 0 {
            final_color_byte = self.get_sprite_color(spr_index, spr_palette);
        } else if bg_index != 0 && spr_index == 0 {
            final_color_byte = self.get_bg_color(bg_index, bg_palette);
        } else {
            // Обработка Sprite 0 Hit
            if is_zero && (self.mask & 0x18) == 0x18 {
                let clipping_active = (self.mask & 0x06) != 0x06;
                if self.cycle < 256 {
                    if self.cycle > 8 || !clipping_active {
                        self.status |= 0x40;
                    }
                }
            }

            if spr_priority {
                final_color_byte = self.get_sprite_color(spr_index, spr_palette);
            } else {
                final_color_byte = self.get_bg_color(bg_index, bg_palette);
            }
        }

        self.screen_buffer[y * 256 + x] = final_color_byte;
    }

    fn evalute_sprite_for_overflow(&mut self) {
        if (self.mask & 0x18) != 0 {
            let mut sprites_on_line = 0;
            let sprite_size = if (self.ctrl & 0x20) != 0 { 16 } else { 8 };

            for i in (0..256).step_by(4) {
                let sprite_y = self.oam[i] as i32;
                let sprite_top = sprite_y;
                let sprite_bottom = sprite_top + sprite_size;

                if (sprite_top..sprite_bottom).contains(&(self.scanline as i32)) {
                    sprites_on_line += 1;
                    if sprites_on_line > 8 {
                        self.status |= 0x20; 
                        break;
                    }
                }
            }
        }
    }

    fn fetch_sprites(&mut self) {
        self.sprite_count = 0;
        let sprite_size = if (self.ctrl & 0x20) != 0 { 16 } else { 8 };

        for i in (0..256).step_by(4) {
            let sprite_y = self.oam[i] as i16;

            if (sprite_y..(sprite_y + sprite_size)).contains(&(self.scanline as i16)) {
                if self.sprite_count == 8 {
                    break;
                }

                let tile_id = self.oam[i + 1];
                let attributes = self.oam[i + 2];
                let x_pos = self.oam[i + 3];

                let mut row = self.scanline - sprite_y ;

                if (attributes & 0x80) != 0 {
                    row = (sprite_size - 1) - row;
                }

                let tile_addr = if sprite_size == 8 {
                    let bank = if (self.ctrl & 0x08) != 0 { 0x1000 } else { 0x0000 };
                    bank + (tile_id as u16 * 16) + row as u16
                } else {
                    let bank = ((tile_id & 0x01) as u16) * 0x1000;
                    let base_tile = (tile_id & 0xFE) as u16;
                    let actual_tile = if row < 8 { base_tile } else { base_tile + 1 };
                    let tile_row = row % 8;
                    bank + (actual_tile * 16) + tile_row as u16
                };

                let pattern_low = self.read_ppu_address(tile_addr);
                let pattern_high = self.read_ppu_address(tile_addr + 8);

                let count = self.sprite_count;
                self.sprite_patterns_low[count] = pattern_low;
                self.sprite_patterns_high[count] = pattern_high;
                self.sprite_attributes[count] = attributes;
                self.sprite_positions_x[count] = x_pos;
                self.sprite_is_zero[count] = i == 0;

                self.sprite_count += 1;
            }
        }
    }

    fn shift_bg_registers(&mut self) {
        if (self.mask & 0x18) != 0 {
            self.bg_shift_pattern_low <<= 1;
            self.bg_shift_pattern_high <<= 1;
            self.bg_shift_attrib_low <<= 1;
            self.bg_shift_attrib_high <<= 1;
        }
    }

    fn get_bg_pixel(&self) -> (u8, u8) {
        if (self.mask & 0x08) == 0 {
            return (0, 0); 
        }

        if self.cycle <= 8 && (self.mask & 0x02) == 0 {
            return (0, 0);
        }

        let bit_mux = 0x8000 >> self.x;

        let p0 = if (self.bg_shift_pattern_low & bit_mux) != 0 { 1 } else { 0 };
        let p1 = if (self.bg_shift_pattern_high & bit_mux) != 0 { 1 } else { 0 };
        let bg_color_index = (p1 << 1) | p0; 

        let a0 = if (self.bg_shift_attrib_low & bit_mux) != 0 { 1 } else { 0 };
        let a1 = if (self.bg_shift_attrib_high & bit_mux) != 0 { 1 } else { 0 };
        let bg_palette_index = (a1 << 1) | a0; 

        (bg_color_index, bg_palette_index)
    }

    fn fetch_bg_data(&mut self) {
        match self.cycle % 8 {
            1 => {
                let address = 0x2000 | (self.v & 0x0FFF);
                self.bg_next_tile_id = self.read_ppu_address(address);
            }
            3 => {
                let address = 0x23C0 | (self.v & 0x0C00) | ((self.v >> 4) & 0x38) | ((self.v >> 2) & 0x07);
                let attribute = self.read_ppu_address(address);
                let shift = ((self.v >> 4) & 0x04) | (self.v & 0x02);
                self.bg_next_attribute = (attribute >> shift) & 0x03;
            }
            5 => {
                let table = if (self.ctrl & 0x10) != 0 { 0x1000 } else { 0x0000 };
                let fine_y = (self.v >> 12) & 0x07;
                let address = table + ((self.bg_next_tile_id as u16) << 4) + fine_y;
                self.bg_next_pattern_low = self.read_ppu_address(address);
            }
            7 => {
                let table = if (self.ctrl & 0x10) != 0 { 0x1000 } else { 0x0000 };
                let fine_y = (self.v >> 12) & 0x07;
                let address = table + ((self.bg_next_tile_id as u16) << 4) + fine_y + 8;
                self.bg_next_pattern_high = self.read_ppu_address(address);
            }
            0 => {
                // Из этой ветки вызов load_bg_shift_registers() удален.
                // Он теперь вызывается строго в step() после завершения сдвигов.
                self.increment_scroll_x();
                if (self.cycle == 256) && (self.mask & 0x18 != 0) {
                    self.increment_scroll_y();
                }
            }
            _ => {}
        }
    }

    fn load_bg_shift_registers(&mut self) {
        self.bg_shift_pattern_low = (self.bg_shift_pattern_low & 0xFF00) | self.bg_next_pattern_low as u16;
        self.bg_shift_pattern_high = (self.bg_shift_pattern_high & 0xFF00) | self.bg_next_pattern_high as u16;

        let a0 = if (self.bg_next_attribute & 0x01) != 0 { 0xFF } else { 0x00 };
        let a1 = if (self.bg_next_attribute & 0x02) != 0 { 0xFF } else { 0x00 };

        self.bg_shift_attrib_low = (self.bg_shift_attrib_low & 0xFF00) | a0 as u16;
        self.bg_shift_attrib_high = (self.bg_shift_attrib_high & 0xFF00) | a1 as u16;
    }

    fn increment_scroll_x(&mut self) {
        if (self.mask & 0x18) == 0 { return; }
        
        if (self.v & 0x001F) == 31 {
            self.v &= !0x001F;
            self.v ^= 0x0400;
        } else {
            self.v += 1;
        }
    }

    fn increment_scroll_y(&mut self) {
        if (self.mask & 0x18) == 0 { return; }
        
        if (self.v & 0x7000) != 0x7000 {
            self.v += 0x1000;
        } else {
            self.v &= !0x7000;
            let mut y = (self.v & 0x03E0) >> 5;
            if y == 29 {
                y = 0;
                self.v ^= 0x0800;
            } else if y == 31 {
                y = 0;
            } else {
                y += 1;
            }
            self.v = (self.v & !0x03E0) | (y << 5);
        }
    }

    fn get_bg_color(&mut self, color_index: u8, palette_index: u8) -> u8 {
        if color_index == 0 {
            return self.read_ppu_address(0x3F00);
        }
        let address = 0x3F00 + (palette_index as u16 * 4) + color_index as u16;
        self.read_ppu_address(address)
    }

    fn get_sprite_pixel(&mut self) -> (u8, u8, bool, bool) {
        if (self.mask & 0x10) == 0 {
            return (0, 0, false, false);
        }

        if self.cycle <= 8 && (self.mask & 0x04) == 0 {
            return (0, 0, false, false);
        }

        for i in 0..self.sprite_count {
            let offset = (self.cycle as i32 - 1) - self.sprite_positions_x[i] as i32;
            if (0..8).contains(&offset) {
                let bit_shift = if (self.sprite_attributes[i] & 0x40) != 0 {
                    offset as u8
                } else {
                    7 - offset as u8
                };

                let p0 = (self.sprite_patterns_low[i] >> bit_shift) & 0x01;
                let p1 = (self.sprite_patterns_high[i] >> bit_shift) & 0x01;
                let color_index = (p1 << 1) | p0;

                if color_index != 0 {
                    let palette_index = (self.sprite_attributes[i] & 0x03) + 4;
                    let priority = (self.sprite_attributes[i] & 0x20) == 0;
                    let is_zero = self.sprite_is_zero[i];
                    return (color_index, palette_index, priority, is_zero);
                }
            }
        }

        (0, 0, false, false)
    }

    fn get_sprite_color(&mut self, color_index: u8, palette_index: u8) -> u8 {
        if color_index == 0 {
            return self.read_ppu_address(0x3F00);
        }
        let address = 0x3F00 + (palette_index as u16 * 4) + color_index as u16;
        self.read_ppu_address(address)
    }

    fn mirror_vram_addr(&self, address: u16) -> usize {
        let vram_addr = (address - 0x2000) & 0x0FFF;
        match self.mirroring {
            MirroringType::Vertical => (vram_addr % 2048) as usize,
            MirroringType::Horizontal => {
                let table = vram_addr / 0x0400;
                let offset = vram_addr % 0x0400;
                let index = match table {
                    0 | 1 => offset,
                    2 | 3 => 1024 + offset,
                    _ => offset,
                };
                index as usize
            }
            _ => (vram_addr % 2048) as usize,
        }
    }

    fn get_palette_index(&self, address: u16) -> usize {
        let mut palette_addr = (address & 0x001F) as usize;
        // Корректное аппаратное зеркалирование палитр:
        // $3F10, $3F14, $3F18, $3F1C зеркалируются на $3F00, $3F04, $3F08, $3F0C соответственно.
        if palette_addr >= 0x10 && (palette_addr % 4 == 0) {
            palette_addr -= 0x10;
        }
        palette_addr
    }

    fn read_ppu_address(&mut self, mut address: u16) -> u8 {
        address &= 0x3FFF;
        match address {
            0x0000..=0x1FFF => {
                self.chr.get(address as usize).copied().unwrap_or(0)
            }
            0x2000..=0x3EFF => {
                let index = self.mirror_vram_addr(address);
                self.vram[index]
            }
            0x3F00..=0x3FFF => {
                let idx = self.get_palette_index(address);
                self.palette[idx]
            }
            _ => 0,
        }
    }

    fn write_ppu_address(&mut self, mut address: u16, data: u8) {
        address &= 0x3FFF;
        match address {
            0x0000..=0x1FFF => {
                if let Some(elem) = self.chr.get_mut(address as usize) {
                    *elem = data;
                }
            }
            0x2000..=0x3EFF => {
                let index = self.mirror_vram_addr(address);
                self.vram[index] = data;
            }
            0x3F00..=0x3FFF => {
                let idx = self.get_palette_index(address);
                self.palette[idx] = data;
            }
            _ => (),
        }
    }

    fn read_ppu_status(&mut self) -> u8 {
        let current_status = (self.status & 0xE0) | (self.ppu_data_buffer & 0x1F);
        self.w = false;
        self.status &= !0x80;
        current_status
    }
        
    fn read_oam_data(&mut self) -> u8 {
        self.oam[self.oam_addr as usize]
    }

    fn increment_vram_addr(&mut self) {
        let inc = if (self.ctrl & 0x04) != 0 { 32 } else { 1 };
        self.v = self.v.wrapping_add(inc) & 0x7FFF;
    }

    fn read_ppu_data(&mut self) -> u8 {
        let addr = self.v & 0x3FFF;
        self.increment_vram_addr();

        if addr <= 0x3EFF {
            let result = self.read_buffer;
            self.read_buffer = self.read_ppu_address(addr);
            result
        } else {
            let result = self.read_ppu_address(addr);
            self.read_buffer = self.read_ppu_address(addr & 0x2FFF);
            result & 0x3F 
        }
    }

    fn write_ppu_ctrl(&mut self, data: u8) {
        let old_nmi = (self.ctrl & 0x80) != 0;
        self.ctrl = data;
        
        self.t &= 0xF3FF;
        self.t |= ((data & 0x03) as u16) << 10;
        
        if !old_nmi && (data & 0x80 != 0) && (self.status & 0x80 != 0) {
            self.nmi_interrupt = true;
        }
    }

    fn write_ppu_mask(&mut self, data: u8) {
        self.mask = data;
    }

    fn write_ppu_addr(&mut self, data: u8) {
        if !self.w {
            self.t = (self.t & 0x00FF) | (((data & 0x3F) as u16) << 8);
            self.w = true;
        } else {
            self.t = (self.t & 0xFF00) | (data as u16);
            self.v = self.t; 
            self.w = false;
        }
    }

    fn write_ppu_scroll(&mut self, data: u8) {
        if !self.w {
            self.x = data & 0x07;
            self.t = (self.t & 0xFFE0) | ((data >> 3) as u16);
            self.w = true;
        } else {
            self.t &= !0x73E0;
            self.t |= ((data & 0x07) as u16) << 12;
            self.t |= ((data & 0xF8) as u16) << 2;
            self.w = false;
        }
    }

    fn write_oam_addr(&mut self, data: u8) {
        self.oam_addr = data;
    }

    fn write_oam_data(&mut self, data: u8) {
        if (self.mask & 0x18) == 0 || self.scanline >= 240 {
            self.oam[self.oam_addr as usize] = data;
            self.oam_addr = self.oam_addr.wrapping_add(1);
        }
    }

    fn write_ppu_data(&mut self, data: u8) {
        self.write_ppu_address(self.v, data);
        self.increment_vram_addr();
    }
    
}