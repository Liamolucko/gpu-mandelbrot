use std::iter;
use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Mul;
use std::ops::Sub;
use std::ops::SubAssign;

#[derive(Default, Clone, Debug)]
pub struct Component {
    int: i32,
    subint: Vec<u32>,
}

impl Add<&Self> for Component {
    type Output = Self;

    fn add(mut self, rhs: &Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl Add for Component {
    type Output = Self;

    fn add(mut self, mut rhs: Self) -> Self::Output {
        if rhs.subint.len() > self.subint.len() {
            rhs += self;
            rhs
        } else {
            self += rhs;
            self
        }
    }
}

impl Add for &Component {
    type Output = Component;

    fn add(self, rhs: Self) -> Self::Output {
        self.clone() + rhs
    }
}

impl AddAssign<&Self> for Component {
    fn add_assign(&mut self, rhs: &Self) {
        if rhs.subint.len() > self.subint.len() {
            self.subint
                .extend(iter::repeat(0).take(rhs.subint.len() - self.subint.len()));
        }

        let mut carry = false;

        for (target, src) in self.subint.iter_mut().zip(rhs.subint.iter().copied()).rev() {
            let (res, new_carry) = target.carrying_add(src, carry);
            carry = new_carry;
            *target = res;
        }

        self.int += rhs.int;
        self.int += carry as i32;
    }
}

impl AddAssign for Component {
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

impl Sub<&Self> for Component {
    type Output = Self;

    fn sub(mut self, rhs: &Self) -> Self::Output {
        self -= rhs;
        self
    }
}

impl Sub for Component {
    type Output = Self;

    fn sub(mut self, mut rhs: Self) -> Self::Output {
        if rhs.subint.len() > self.subint.len() {
            rhs -= self;
            rhs
        } else {
            self -= rhs;
            self
        }
    }
}

impl SubAssign<&Self> for Component {
    fn sub_assign(&mut self, rhs: &Self) {
        if rhs.subint.len() > self.subint.len() {
            self.subint
                .extend(iter::repeat(0).take(rhs.subint.len() - self.subint.len()));
        }

        let mut carry = false;

        for (target, src) in self.subint.iter_mut().zip(rhs.subint.iter().copied()).rev() {
            let (res, new_carry) = target.borrowing_sub(src, carry);
            carry = new_carry;
            *target = res;
        }

        self.int -= rhs.int;
        self.int -= carry as i32;
    }
}

impl SubAssign for Component {
    fn sub_assign(&mut self, rhs: Self) {
        *self -= &rhs;
    }
}

impl Mul for &Component {
    type Output = Component;

    fn mul(self, rhs: Self) -> Self::Output {
        let out_subint_len = self.subint.len() + rhs.subint.len();

        let mut out = Component {
            int: 0,
            subint: vec![0; out_subint_len],
        };

        // The carry from the actual multiplication
        let mut carry = 0;
        // The carry from adding to the result
        let mut bitcarry = false;

        for (i, digit_a) in self.subint.iter().copied().rev().enumerate() {
            for (digit_b, target) in rhs
                .subint
                .iter()
                .copied()
                .rev()
                .chain(iter::repeat(0))
                .zip(out.subint[..=out_subint_len - i].iter_mut().rev())
            {
                let (res, new_carry) = digit_a.carrying_mul(digit_b, carry);
                carry = new_carry;
                let (new_target, new_bitcarry) = target.carrying_add(res, bitcarry);
                *target = new_target;
                bitcarry = new_bitcarry;
            }
        }

        out
    }
}

impl Mul for Component {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        &self * &rhs
    }
}

impl From<f32> for Component {
    fn from(num: f32) -> Self {
        if num.is_infinite() {
            panic!("`Component`s cannot be infinite");
        } else if num.is_nan() {
            panic!("`Component`s cannot be NaN");
        }

        let int = num.floor();
        let subint = num - num.floor();

        let exponent = (int.to_bits() >> 23) as u8;
        // The mantissa of a float is effectively 24 bits,
        // so you can only shift it up at most 7 bits whilst having it fit in an i32 (since the first bit is a sign bit)
        if exponent as i16 - 127 > 7 {
            panic!("{} is too large to fit in a `Component`", num);
        }
        // We've now checked that this will fit.
        let int = int as i32;

        let bits = subint.to_bits();
        let exponent = (bits >> 23) as u8;
        let fraction = 1 << 31 | (bits & (u32::MAX >> 9)) << 8;

        // How many bits the subint is offset from the start of the subint portion.
        // offset = -1 - (exponent - 127)
        //        = 126 - exponent
        let offset = 126 - exponent;

        // The number of bits needed for the subint.
        // -1 - (exponent - 127) + 24 - trailing_zeros
        // 126 - exponent + 24 - trailing_zeros
        // 150 - exponent - trailing_zeros
        let subint_bits =
            usize::from(offset + 24) - fraction.trailing_zeros().clamp(0, 23) as usize;

        let mut out = Self {
            int,
            subint: vec![0; subint_bits / 50],
        };

        let idx = usize::from(offset) / 32;
        let suboffset = offset % 32;
        out.subint[idx] = fraction >> suboffset;
        // We won't have allocated room for the next digit if we didn't need it.
        if idx + 1 < out.subint.len() {
            out.subint[idx + 1] = fraction << (32 - suboffset);
        }

        out
    }
}

impl From<i32> for Component {
    fn from(int: i32) -> Self {
        Self {
            int,
            subint: vec![],
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Complex {
    pub real: Component,
    pub imag: Component,
}

impl Complex {
    pub fn square(&self) -> Self {
        let real = (self.real.clone() + &self.imag) * (self.real.clone() - &self.imag);
        let mut imag = self.real.clone() + &self.imag;
        imag += imag.clone();
        Self { real, imag }
    }
}
