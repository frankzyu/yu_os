macro_rules! read_csr {
    ($csr_number:expr, $asm_fn: ident) => {
        /// Reads the CSR
        #[inline]
        unsafe fn _read() -> usize {
            let r: usize;
            core::arch::asm!("csrrs {0}, {1}, x0", out(reg) r, const $csr_number);
            r
        }
    };
}

macro_rules! read_csr_as {
    ($register:ident, $csr_number:expr, $asm_fn: ident) => {
        read_csr!($csr_number, $asm_fn);
        #[inline]
        pub fn read() -> $register {
            $register {
                bits: unsafe { _read() },
            }
        }
    };
}

macro_rules! read_csr_as_usize {
    ($csr_number:expr, $asm_fn: ident) => {
        read_csr!($csr_number, $asm_fn);
        #[inline]
        pub fn read() -> usize {
            unsafe { _read() }
        }
    };
}


macro_rules! write_csr {
    ($csr_number:expr, $asm_fn: ident) => {
        #[inline]
        #[allow(unused_variables)]
        unsafe fn _write(bits: usize) {
            core::arch::asm!("csrrw x0, {1}, {0}", in(reg) bits, const $csr_number);
        }
    };
}


macro_rules! write_csr_as_usize {
    ($csr_number:expr, $asm_fn: ident) => {
        write_csr!($csr_number, $asm_fn);
        #[inline]
        pub fn write(bits: usize) {
            unsafe { _write(bits) }
        }
    };
}

macro_rules! set {
    ($csr_number:expr, $asm_fn: ident) => {
        /// Set the CSR
        #[inline]
        #[allow(unused_variables)]
        unsafe fn _set(bits: usize) {
            core::arch::asm!("csrrs x0, {1}, {0}", in(reg) bits, const $csr_number);
        }
    };
}

macro_rules! clear {
    ($csr_number:expr, $asm_fn: ident) => {
        /// Clear the CSR
        #[inline]
        #[allow(unused_variables)]
        unsafe fn _clear(bits: usize) {
            core::arch::asm!("csrrc x0, {1}, {0}", in(reg) bits, const $csr_number);
        }
    };
}

macro_rules! set_csr {
    ($(#[$attr:meta])*, $set_field:ident, $e:expr) => {
        $(#[$attr])*
        #[inline]
        pub unsafe fn $set_field() {
            _set($e);
        }
    };
}

macro_rules! clear_csr {
    ($(#[$attr:meta])*, $clear_field:ident, $e:expr) => {
        $(#[$attr])*
        #[inline]
        pub unsafe fn $clear_field() {
            _clear($e);
        }
    };
}

macro_rules! set_clear_csr {
    ($(#[$attr:meta])*, $set_field:ident, $clear_field:ident, $e:expr) => {
        set_csr!($(#[$attr])*, $set_field, $e);
        clear_csr!($(#[$attr])*, $clear_field, $e);
    }
}

macro_rules! read_csr_rv32 {
    ($csr_number:expr, $asm_fn: ident) => {
        #[inline]
        unsafe fn _read() -> usize {
            let r: usize;
            core::arch::asm!("csrrs {0}, {1}, x0", out(reg) r, const $csr_number);
            r
        }
    };
}

macro_rules! read_csr_as_usize_rv32 {
    ($csr_number:expr, $asm_fn: ident) => {
        read_csr_rv32!($csr_number, $asm_fn);
        #[inline]
        pub fn read() -> usize {
            unsafe { _read() }
        }
    };
}

macro_rules! read_composite_csr {
    ($hi:expr, $lo:expr) => {
        #[inline]
        pub fn read64() -> u64 {
            let hi = $hi;
            let lo = $lo;
            ((hi as u64) << 32) | (lo as u64)
        }
    };
}