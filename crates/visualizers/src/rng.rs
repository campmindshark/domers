use domers_core::Rgb;

#[derive(Clone, Debug)]
pub(crate) struct DotNetRandom {
    seed_array: [i32; 56],
    inext: usize,
    inextp: usize,
}

impl DotNetRandom {
    const MBIG: i32 = 2_147_483_647;
    const MSEED: i32 = 161_803_398;

    pub(crate) fn new(seed: i32) -> Self {
        let subtraction = if seed == i32::MIN {
            i32::MAX
        } else {
            seed.abs()
        };
        let mut seed_array = [0; 56];
        let mut mj = Self::MSEED - subtraction;
        if mj < 0 {
            mj += Self::MBIG;
        }
        seed_array[55] = mj;
        let mut mk = 1;
        for i in 1..55 {
            let ii = (21 * i) % 55;
            seed_array[ii] = mk;
            mk = mj - mk;
            if mk < 0 {
                mk += Self::MBIG;
            }
            mj = seed_array[ii];
        }
        for _ in 0..4 {
            for i in 1..56 {
                seed_array[i] -= seed_array[1 + (i + 30) % 55];
                if seed_array[i] < 0 {
                    seed_array[i] += Self::MBIG;
                }
            }
        }
        Self {
            seed_array,
            inext: 0,
            inextp: 21,
        }
    }

    pub(crate) fn internal_sample(&mut self) -> i32 {
        self.inext += 1;
        if self.inext >= 56 {
            self.inext = 1;
        }
        self.inextp += 1;
        if self.inextp >= 56 {
            self.inextp = 1;
        }
        let mut ret = self.seed_array[self.inext] - self.seed_array[self.inextp];
        if ret == Self::MBIG {
            ret -= 1;
        }
        if ret < 0 {
            ret += Self::MBIG;
        }
        self.seed_array[self.inext] = ret;
        ret
    }

    pub(crate) fn next_double(&mut self) -> f64 {
        f64::from(self.internal_sample()) * (1.0 / f64::from(Self::MBIG))
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "Spectrum truncates Random.NextDouble multiplied by the byte brightness cap"
    )]
    pub(crate) fn next_color(&mut self, brightness_byte: i32) -> Rgb {
        let blue = (self.next_double() * f64::from(brightness_byte)) as u8;
        let green = (self.next_double() * f64::from(brightness_byte)) as u8;
        let red = (self.next_double() * f64::from(brightness_byte)) as u8;
        Rgb {
            r: red,
            g: green,
            b: blue,
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "Spectrum .NET Random.Next(min,max) truncates Sample()*range to an int"
    )]
    pub(crate) fn next_int(&mut self, min_value: i32, max_value: i32) -> i32 {
        let range = i64::from(max_value) - i64::from(min_value);
        (self.next_double() * range as f64) as i32 + min_value
    }

    /// Mirrors C# `Random.Next()`, returning a value in `[0, i32::MAX)`.
    pub(crate) fn next(&mut self) -> i32 {
        self.internal_sample()
    }
}
