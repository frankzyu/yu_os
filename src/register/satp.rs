//! satp register


use crate::address::Frame;
use crate::bit_field::BitField;

/// satp register
#[derive(Clone, Copy, Debug)]
pub struct Satp {
    bits: usize,
}

impl Satp {
    /// Returns the contents of the register as raw bits
    #[inline]
    pub fn bits(&self) -> usize {
        self.bits
    }
    /// Current address-translation scheme
    #[inline]

    pub fn mode(&self) -> Mode {
        match self.bits.get_bits(60..64) {
            0 => Mode::Bare,
            8 => Mode::Sv39,
            9 => Mode::Sv48,
            10 => Mode::Sv57,
            11 => Mode::Sv64,
            _ => unreachable!(),
        }
    }

    /// Address space identifier

    /// Address space identifier
    #[inline]
    pub fn asid(&self) -> usize {
        self.bits.get_bits(44..60)
    }




    #[inline]
    pub fn ppn(&self) -> usize {
        self.bits.get_bits(0..44)
    }

    /// Physical frame
    #[inline]
    pub fn frame(&self) -> Frame {
        Frame::of_ppn(self.ppn())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Bare = 0,
    Sv39 = 8,
    Sv48 = 9,
    Sv57 = 10,
    Sv64 = 11,
}

read_csr_as!(Satp, 0x180, __read_satp);
write_csr_as_usize!(0x180, __write_satp);


#[inline]

pub unsafe fn set(mode: Mode, asid: usize, ppn: usize) {
    let mut bits = 0usize;
    bits.set_bits(60..64, mode as usize);
    bits.set_bits(44..60, asid);
    bits.set_bits(0..44, ppn);
    _write(bits);
}