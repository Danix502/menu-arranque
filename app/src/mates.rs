//! Cuentas en coma flotante sin `std` (libm) y un generador de azar.

pub const PI: f32 = core::f32::consts::PI;

pub fn sin(x: f32) -> f32 { libm::sinf(x) }
pub fn cos(x: f32) -> f32 { libm::cosf(x) }
pub fn exp(x: f32) -> f32 { libm::expf(x) }
pub fn pow(x: f32, y: f32) -> f32 { libm::powf(x, y) }
pub fn floor(x: f32) -> f32 { libm::floorf(x) }
pub fn ceil(x: f32) -> f32 { libm::ceilf(x) }
pub fn abs(x: f32) -> f32 { libm::fabsf(x) }
/// Resto siempre positivo (como `((a % b) + b) % b` en la maqueta)
pub fn resto(a: f32, b: f32) -> f32 { a - b * floor(a / b) }

pub fn lim(v: f32) -> f32 { v.clamp(0.0, 1.0) }
pub fn suave(v: f32) -> f32 { let v = lim(v); v * v * (3.0 - 2.0 * v) }
pub fn salida(v: f32) -> f32 { 1.0 - pow(1.0 - lim(v), 3.0) }
/// Acerca `v` a `obj` sin depender de los fotogramas por segundo
pub fn aprox(v: f32, obj: f32, vel: f32, dt: f32) -> f32 { v + (obj - v) * (1.0 - exp(-vel * dt)) }

/// xorshift64*: basta para partículas
pub struct Azar(u64);

impl Azar {
    pub fn new(semilla: u64) -> Self { Azar(semilla | 1) }
    pub fn sig(&mut self) -> f32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn entre(&mut self, a: f32, b: f32) -> f32 { a + self.sig() * (b - a) }
}
