//! Noise generation matching LCE reference (ImprovedNoise, PerlinNoise)

use mclre_core::GameRandom;

/// Improved noise (Perlin simplex-like) matching reference exactly
#[derive(Debug, Clone)]
pub struct ImprovedNoise {
    p: [i32; 512],
    xo: f64,
    yo: f64,
    zo: f64,
}

impl ImprovedNoise {
    /// Create with a random seed
    pub fn new(random: &mut GameRandom) -> Self {
        let mut noise = Self {
            p: [0; 512],
            xo: 0.0,
            yo: 0.0,
            zo: 0.0,
        };
        noise.init(random);
        noise
    }

    fn init(&mut self, random: &mut GameRandom) {
        self.xo = random.next_double() * 256.0;
        self.yo = random.next_double() * 256.0;
        self.zo = random.next_double() * 256.0;

        for i in 0..256 {
            self.p[i] = i as i32;
        }
        for i in 0..256 {
            let j = random.next_int_bound(256 - i) + i;
            let tmp = self.p[i as usize];
            self.p[i as usize] = self.p[j as usize];
            self.p[j as usize] = tmp;
            self.p[i as usize + 256] = self.p[i as usize];
        }
    }

    /// 3D noise function
    pub fn noise(&self, x: f64, y: f64, z: f64) -> f64 {
        let x = x + self.xo;
        let y = y + self.yo;
        let z = z + self.zo;

        let xf = if x < 0.0 { x.floor() as i32 } else { x as i32 };
        let yf = if y < 0.0 { y.floor() as i32 } else { y as i32 };
        let zf = if z < 0.0 { z.floor() as i32 } else { z as i32 };

        let x = x - xf as f64;
        let y = y - yf as f64;
        let z = z - zf as f64;

        let X = xf & 255;
        let Y = yf & 255;
        let Z = zf & 255;

        let u = Self::fade(x);
        let v = Self::fade(y);
        let w = Self::fade(z);

        let A = self.p[X as usize] + Y;
        let AA = self.p[A as usize] + Z;
        let AB = self.p[A as usize + 1] + Z;
        let B = self.p[(X + 1) as usize] + Y;
        let BA = self.p[B as usize] + Z;
        let BB = self.p[B as usize + 1] + Z;

        Self::lerp(
            w,
            Self::lerp(
                v,
                Self::lerp(
                    u,
                    self.grad(self.p[AA as usize], x, y, z),
                    self.grad(self.p[BA as usize], x - 1.0, y, z),
                ),
                Self::lerp(
                    u,
                    self.grad(self.p[AB as usize], x, y - 1.0, z),
                    self.grad(self.p[BB as usize], x - 1.0, y - 1.0, z),
                ),
            ),
            Self::lerp(
                v,
                Self::lerp(
                    u,
                    self.grad(self.p[AA as usize + 1], x, y, z - 1.0),
                    self.grad(self.p[BA as usize + 1], x - 1.0, y, z - 1.0),
                ),
                Self::lerp(
                    u,
                    self.grad(self.p[AB as usize + 1], x, y - 1.0, z - 1.0),
                    self.grad(self.p[BB as usize + 1], x - 1.0, y - 1.0, z - 1.0),
                ),
            ),
        )
    }

    /// 2D noise function
    pub fn noise2d(&self, x: f64, y: f64) -> f64 {
        self.noise(x, y, 0.0)
    }

    fn fade(t: f64) -> f64 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }
    fn lerp(t: f64, a: f64, b: f64) -> f64 {
        a + t * (b - a)
    }
    fn grad2(&self, hash: i32, x: f64, z: f64) -> f64 {
        let h = hash & 15;
        let u = if (h & 8) != 0 { -x } else { x };
        let v = if h < 4 {
            0.0
        } else if h == 12 || h == 14 {
            x
        } else {
            z
        };
        (if (h & 1) == 0 { u } else { -u }) + (if (h & 2) == 0 { v } else { -v })
    }
    fn grad(&self, hash: i32, x: f64, y: f64, z: f64) -> f64 {
        let h = hash & 15;
        let u = if h < 8 { x } else { y };
        let v = if h < 4 {
            y
        } else if h == 12 || h == 14 {
            x
        } else {
            z
        };
        (if (h & 1) == 0 { u } else { -u }) + (if (h & 2) == 0 { v } else { -v })
    }

    /// Add noise to a buffer (optimized for region generation)
    pub fn add(
        &self,
        buffer: &mut [f64],
        x: f64,
        y: f64,
        z: f64,
        x_size: i32,
        y_size: i32,
        z_size: i32,
        x_scale: f64,
        y_scale: f64,
        z_scale: f64,
        pow: f64,
    ) {
        if y_size == 1 {
            let scale = 1.0 / pow;
            let mut pp = 0;
            for xx in 0..x_size {
                let xx_f = x + (xx as f64) * x_scale + self.xo;
                let xf = if xx_f < 0.0 {
                    xx_f.floor() as i32
                } else {
                    xx_f as i32
                };
                let X = xf & 255;
                let x_rel = xx_f - xf as f64;
                let u = Self::fade(x_rel);
                for zz in 0..z_size {
                    let zz_f = z + (zz as f64) * z_scale + self.zo;
                    let zf = if zz_f < 0.0 {
                        zz_f.floor() as i32
                    } else {
                        zz_f as i32
                    };
                    let Z = zf & 255;
                    let z_rel = zz_f - zf as f64;
                    let w = Self::fade(z_rel);
                    let A = self.p[X as usize];
                    let AA = self.p[A as usize] + Z;
                    let B = self.p[(X + 1) as usize];
                    let BA = self.p[B as usize] + Z;
                    let vv0 = Self::lerp(
                        u,
                        self.grad2(self.p[AA as usize], x_rel, z_rel),
                        self.grad(self.p[BA as usize], x_rel - 1.0, 0.0, z_rel),
                    );
                    let vv2 = Self::lerp(
                        u,
                        self.grad(self.p[AA as usize + 1], x_rel, 0.0, z_rel - 1.0),
                        self.grad(self.p[BA as usize + 1], x_rel - 1.0, 0.0, z_rel - 1.0),
                    );
                    let val = Self::lerp(w, vv0, vv2);
                    buffer[pp] += val * scale;
                    pp += 1;
                }
            }
            return;
        }
        let scale = 1.0 / pow;
        let mut pp = 0;
        let mut y_old = -1;
        let mut A = 0;
        let mut AA = 0;
        let mut AB = 0;
        let mut B = 0;
        let mut BA = 0;
        let mut BB = 0;
        let mut vv0 = 0.0;
        let mut vv1 = 0.0;
        let mut vv2 = 0.0;
        let mut vv3 = 0.0;
        for xx in 0..x_size {
            let xx_f = x + (xx as f64) * x_scale + self.xo;
            let xf = if xx_f < 0.0 {
                xx_f.floor() as i32
            } else {
                xx_f as i32
            };
            let X = xf & 255;
            let x_rel = xx_f - xf as f64;
            let u = Self::fade(x_rel);
            for zz in 0..z_size {
                let zz_f = z + (zz as f64) * z_scale + self.zo;
                let zf = if zz_f < 0.0 {
                    zz_f.floor() as i32
                } else {
                    zz_f as i32
                };
                let Z = zf & 255;
                let z_rel = zz_f - zf as f64;
                let w = Self::fade(z_rel);
                for yy in 0..y_size {
                    let yy_f = y + (yy as f64) * y_scale + self.yo;
                    let yf = if yy_f < 0.0 {
                        yy_f.floor() as i32
                    } else {
                        yy_f as i32
                    };
                    let Y = yf & 255;
                    let y_rel = yy_f - yf as f64;
                    let v = Self::fade(y_rel);
                    if yy == 0 || Y != y_old {
                        y_old = Y;
                        A = self.p[X as usize] + Y;
                        AA = self.p[A as usize] + Z;
                        AB = self.p[A as usize + 1] + Z;
                        B = self.p[(X + 1) as usize] + Y;
                        BA = self.p[B as usize] + Z;
                        BB = self.p[B as usize + 1] + Z;
                        vv0 = Self::lerp(
                            u,
                            self.grad(self.p[AA as usize], x_rel, y_rel, z_rel),
                            self.grad(self.p[BA as usize], x_rel - 1.0, y_rel, z_rel),
                        );
                        vv1 = Self::lerp(
                            u,
                            self.grad(self.p[AB as usize], x_rel, y_rel - 1.0, z_rel),
                            self.grad(self.p[BB as usize], x_rel - 1.0, y_rel - 1.0, z_rel),
                        );
                        vv2 = Self::lerp(
                            u,
                            self.grad(self.p[AA as usize + 1], x_rel, y_rel, z_rel - 1.0),
                            self.grad(self.p[BA as usize + 1], x_rel - 1.0, y_rel, z_rel - 1.0),
                        );
                        vv3 = Self::lerp(
                            u,
                            self.grad(self.p[AB as usize + 1], x_rel, y_rel - 1.0, z_rel - 1.0),
                            self.grad(
                                self.p[BB as usize + 1],
                                x_rel - 1.0,
                                y_rel - 1.0,
                                z_rel - 1.0,
                            ),
                        );
                    }
                    let v0 = Self::lerp(v, vv0, vv1);
                    let v1 = Self::lerp(v, vv2, vv3);
                    let val = Self::lerp(w, v0, v1);
                    buffer[pp] += val * scale;
                    pp += 1;
                }
            }
        }
    }
}

/// Perlin noise (multi-octave)
#[derive(Debug, Clone)]
pub struct PerlinNoise {
    levels: i32,
    noise_levels: Vec<ImprovedNoise>,
}

impl PerlinNoise {
    pub fn new(random: &mut GameRandom, levels: i32) -> Self {
        let mut noise_levels = Vec::with_capacity(levels as usize);
        for _ in 0..levels {
            noise_levels.push(ImprovedNoise::new(random));
        }
        Self {
            levels,
            noise_levels,
        }
    }
    pub fn get_value(&self, x: f64, y: f64) -> f64 {
        let mut value = 0.0;
        let mut pow = 1.0;
        for i in 0..self.levels {
            value += self.noise_levels[i as usize].noise2d(x * pow, y * pow) / pow;
            pow /= 2.0;
        }
        value
    }
    pub fn get_value_3d(&self, x: f64, y: f64, z: f64) -> f64 {
        let mut value = 0.0;
        let mut pow = 1.0;
        for i in 0..self.levels {
            value += self.noise_levels[i as usize].noise(x * pow, y * pow, z * pow) / pow;
            pow /= 2.0;
        }
        value
    }
    pub fn get_region(
        &self,
        buffer: &mut [f64],
        x: i32,
        y: i32,
        z: i32,
        x_size: i32,
        y_size: i32,
        z_size: i32,
        x_scale: f64,
        y_scale: f64,
        z_scale: f64,
    ) {
        let total = (x_size * y_size * z_size) as usize;
        if buffer.len() < total {
            // Buffer too small, fill what we can
            for i in 0..buffer.len() {
                buffer[i] = 0.0;
            }
        } else {
            for i in 0..total {
                buffer[i] = 0.0;
            }
        }
        let mut pow = 1.0;
        for i in 0..self.levels {
            let xx = x as f64 * pow * x_scale;
            let yy = y as f64 * pow * y_scale;
            let zz = z as f64 * pow * z_scale;
            let xb = (xx.floor() as i64);
            let zb = (zz.floor() as i64);
            let xx = xx - xb as f64 + xb as f64;
            let zz = zz - zb as f64 + zb as f64;
            let xb_wrapped = xb & 0xFFFFFF;
            let zb_wrapped = zb & 0xFFFFFF;
            let xx = xx + xb_wrapped as f64;
            let zz = zz + zb_wrapped as f64;
            self.noise_levels[i as usize].add(
                buffer,
                xx,
                yy,
                zz,
                x_size,
                y_size,
                z_size,
                x_scale * pow,
                y_scale * pow,
                z_scale * pow,
                pow,
            );
            pow /= 2.0;
        }
    }
    pub fn get_region_2d(
        &self,
        buffer: &mut [f64],
        x: i32,
        z: i32,
        x_size: i32,
        z_size: i32,
        x_scale: f64,
        z_scale: f64,
        pow: f64,
    ) {
        self.get_region(buffer, x, 10, z, x_size, 1, z_size, x_scale, 1.0, z_scale);
    }
}
