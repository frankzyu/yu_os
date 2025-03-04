use core::ops::{Bound, Range, RangeBounds};

pub trait BitField {

    const BIT_LENGTH: usize;

    fn get_bit(&self, bit: usize) -> bool;

    fn get_bits<T: RangeBounds<usize>>(&self, range: T) -> Self;

    fn set_bit(&mut self, bit: usize, value: bool) -> &mut Self;

    fn set_bits<T: RangeBounds<usize>>(&mut self, range: T, value: Self) -> &mut Self;
}

pub trait BitArray<T: BitField> {
    fn bit_length(&self) -> usize;

    fn get_bit(&self, bit: usize) -> bool;

    fn get_bits<U: RangeBounds<usize>>(&self, range: U) -> T;

    fn set_bit(&mut self, bit: usize, value: bool);

    fn set_bits<U: RangeBounds<usize>>(&mut self, range: U, value: T);
}
macro_rules! bitfield_numeric_impl {
    ($($t:ty)*) => ($(
        impl BitField for $t {
            const BIT_LENGTH: usize = ::core::mem::size_of::<Self>() as usize * 8;
            #[track_caller]
            #[inline]
            fn get_bit(&self, bit: usize) -> bool {
                assert!(bit < Self::BIT_LENGTH);

                (*self & (1 << bit)) != 0
            }
            #[track_caller]
            #[inline]
            fn get_bits<T: RangeBounds<usize>>(&self, range: T) -> Self {
                let range = to_regular_range(&range, Self::BIT_LENGTH);
                assert!(range.start < Self::BIT_LENGTH);
                assert!(range.end <= Self::BIT_LENGTH);
                assert!(range.start < range.end);
                let bits = *self << (Self::BIT_LENGTH - range.end) >> (Self::BIT_LENGTH - range.end);
                bits >> range.start
            }

            #[track_caller]
            #[inline]
            fn set_bit(&mut self, bit: usize, value: bool) -> &mut Self {
                assert!(bit < Self::BIT_LENGTH);

                if value {
                    *self |= 1 << bit;
                } else {
                    *self &= !(1 << bit);
                }

                self
            }

            #[track_caller]
            #[inline]
            fn set_bits<T: RangeBounds<usize>>(&mut self, range: T, value: Self) -> &mut Self {
                let range = to_regular_range(&range, Self::BIT_LENGTH);

                assert!(range.start < Self::BIT_LENGTH);
                assert!(range.end <= Self::BIT_LENGTH);
                assert!(range.start < range.end);
                assert!(value << (Self::BIT_LENGTH - (range.end - range.start)) >>
                        (Self::BIT_LENGTH - (range.end - range.start)) == value,
                        "value does not fit into bit range");

                let bitmask: Self = !(!0 << (Self::BIT_LENGTH - range.end) >>
                                    (Self::BIT_LENGTH - range.end) >>
                                    range.start << range.start);

                // set bits
                *self = (*self & bitmask) | (value << range.start);

                self
            }
        }
    )*)
}

bitfield_numeric_impl! { u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize }

impl<T: BitField> BitArray<T> for [T] {
    #[inline]
    fn bit_length(&self) -> usize {
        self.len() * T::BIT_LENGTH
    }

    #[track_caller]
    #[inline]
    fn get_bit(&self, bit: usize) -> bool {
        let slice_index = bit / T::BIT_LENGTH;
        let bit_index = bit % T::BIT_LENGTH;
        self[slice_index].get_bit(bit_index)
    }

    #[track_caller]
    #[inline]
    fn get_bits<U: RangeBounds<usize>>(&self, range: U) -> T {
        let range = to_regular_range(&range, self.bit_length());

        assert!(range.len() <= T::BIT_LENGTH);

        let slice_start = range.start / T::BIT_LENGTH;
        let slice_end = range.end / T::BIT_LENGTH;
        let bit_start = range.start % T::BIT_LENGTH;
        let bit_end = range.end % T::BIT_LENGTH;
        let len = range.len();

        assert!(slice_end - slice_start <= 1);

        if slice_start == slice_end {
            self[slice_start].get_bits(bit_start..bit_end)
        } else if bit_end == 0 {
            self[slice_start].get_bits(bit_start..T::BIT_LENGTH)
        } else {
            let mut ret = self[slice_start].get_bits(bit_start..T::BIT_LENGTH);
            ret.set_bits(
                (T::BIT_LENGTH - bit_start)..len,
                self[slice_end].get_bits(0..bit_end),
            );
            ret
        }
    }

    #[track_caller]
    #[inline]
    fn set_bit(&mut self, bit: usize, value: bool) {
        let slice_index = bit / T::BIT_LENGTH;
        let bit_index = bit % T::BIT_LENGTH;
        self[slice_index].set_bit(bit_index, value);
    }

    #[track_caller]
    #[inline]
    fn set_bits<U: RangeBounds<usize>>(&mut self, range: U, value: T) {
        let range = to_regular_range(&range, self.bit_length());

        assert!(range.len() <= T::BIT_LENGTH);

        let slice_start = range.start / T::BIT_LENGTH;
        let slice_end = range.end / T::BIT_LENGTH;
        let bit_start = range.start % T::BIT_LENGTH;
        let bit_end = range.end % T::BIT_LENGTH;

        assert!(slice_end - slice_start <= 1);

        if slice_start == slice_end {
            self[slice_start].set_bits(bit_start..bit_end, value);
        } else if bit_end == 0 {
            self[slice_start].set_bits(bit_start..T::BIT_LENGTH, value);
        } else {
            self[slice_start].set_bits(
                bit_start..T::BIT_LENGTH,
                value.get_bits(0..T::BIT_LENGTH - bit_start),
            );
            self[slice_end].set_bits(
                0..bit_end,
                value.get_bits(T::BIT_LENGTH - bit_start..T::BIT_LENGTH),
            );
        }
    }
}

fn to_regular_range<T: RangeBounds<usize>>(generic_rage: &T, bit_length: usize) -> Range<usize> {
    let start = match generic_rage.start_bound() {
        Bound::Excluded(&value) => value + 1,
        Bound::Included(&value) => value,
        Bound::Unbounded => 0,
    };
    let end = match generic_rage.end_bound() {
        Bound::Excluded(&value) => value,
        Bound::Included(&value) => value + 1,
        Bound::Unbounded => bit_length,
    };
    start..end
}