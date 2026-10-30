//! Dibujo por software: las máscaras de recursos.rs sobre un lienzo de píxeles 0x00RRGGBB
//! (el mismo orden que BltPixel de GOP), con escala bilineal y mezcla normal o aditiva.
//! Dentro de los bucles de píxeles todo va en enteros (coma fija 16.16 y pesos de 0 a 256):
//! el destino x86_64-unknown-uefi de Rust no usa SSE y la coma flotante va por software,
//! unas diez veces más lenta. Los `f32` solo se usan una vez por llamada.

use alloc::vec;
use alloc::vec::Vec;

use crate::mates::{abs, ceil, cos, floor, sin};
use crate::recursos::DATOS;

/// Un sprite dentro de recursos.bin: una máscara (un byte de alfa por píxel; el color lo pone
/// quien dibuja) o datos ya en bytes (texturas, envolventes y paletas del fuego). (cx, cy) es
/// su ancla: el centro, o en los textos el centro del texto y su línea base.
#[derive(Clone, Copy)]
pub struct Sprite {
    pub inicio: usize,
    pub w: usize,
    pub h: usize,
    pub cx: f32,
    pub cy: f32,
}

impl Sprite {
    pub fn bytes(&self) -> &'static [u8] {
        &DATOS[self.inicio..self.inicio + self.w * self.h]
    }
    /// Todo el sprite como recorte
    pub const fn entero(&self) -> [i32; 4] {
        [0, 0, self.w as i32, self.h as i32]
    }
}

pub type Color = [f32; 3];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mezcla {
    Normal,
    Suma,
}

/// 1.0 en coma fija 16.16
const UNO: i32 = 1 << 16;

fn fijo(v: f32) -> i32 {
    floor(v * 65536.0) as i32
}
/// Alfa de 0 a 256
fn alfa256(a: f32) -> i32 {
    (a.clamp(0.0, 1.0) * 256.0 + 0.5) as i32
}
fn color_entero(c: &Color) -> [i32; 3] {
    c.map(|v| (v.clamp(0.0, 255.0) + 0.5) as i32)
}

#[inline]
fn entero_abajo(v: f32) -> i32 {
    let i = v as i32;
    if v < i as f32 { i - 1 } else { i }
}

/// Byte de máscara de 0..255 a 0..256 (así 255 cubre del todo)
#[inline]
fn byte(m: &[u8], i: usize) -> i32 {
    let b = m[i] as i32;
    b + (b >> 7)
}

/// Muestra bilineal de una máscara en (u, v) 16.16 (coordenadas del sprite; el centro de
/// un píxel está en +0.5). Fuera del recorte repite el borde. Devuelve 0..65536.
#[inline]
fn muestra(m: &[u8], w: usize, rec: [i32; 4], u: i32, v: i32) -> i32 {
    let (fu, fv) = (u - UNO / 2, v - UNO / 2);
    let (iu, iv) = (fu >> 16, fv >> 16);
    let (tx, ty) = ((fu >> 8) & 255, (fv >> 8) & 255);
    let xa = iu.clamp(rec[0], rec[2] - 1) as usize;
    let xb = (iu + 1).clamp(rec[0], rec[2] - 1) as usize;
    let ya = iv.clamp(rec[1], rec[3] - 1) as usize * w;
    let yb = (iv + 1).clamp(rec[1], rec[3] - 1) as usize * w;
    let arriba = byte(m, ya + xa) * (256 - tx) + byte(m, ya + xb) * tx;
    let abajo = byte(m, yb + xa) * (256 - tx) + byte(m, yb + xb) * tx;
    (arriba * (256 - ty) + abajo * ty) >> 8
}

/// Mezcla un color con una cobertura de 0..65536
#[inline]
fn mezclar(d: u32, c: &[i32; 3], cob: i32, mezcla: Mezcla) -> u32 {
    let (dr, dg, db) = (((d >> 16) & 255) as i32, ((d >> 8) & 255) as i32, (d & 255) as i32);
    let (r, g, b) = match mezcla {
        Mezcla::Normal => (dr + (((c[0] - dr) * cob) >> 16), dg + (((c[1] - dg) * cob) >> 16), db + (((c[2] - db) * cob) >> 16)),
        Mezcla::Suma => (
            (dr + ((c[0] * cob) >> 16)).min(255),
            (dg + ((c[1] * cob) >> 16)).min(255),
            (db + ((c[2] * cob) >> 16)).min(255),
        ),
    };
    ((r.max(0) as u32) << 16) | ((g.max(0) as u32) << 8) | b.max(0) as u32
}

/// Raíz cuadrada entera
pub fn raiz(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    let mut x = 1u64 << (64 - n.leading_zeros()).div_ceil(2);
    loop {
        let y = (x + n / x) / 2;
        if y >= x {
            return x;
        }
        x = y;
    }
}

/// La pantalla, a 1920×1080
pub struct Lienzo {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u32>,
    /// Foto de un fotograma, para el glitch de toda la pantalla (se reserva al usarla)
    copia: Vec<u32>,
}

impl Lienzo {
    pub fn new(w: usize, h: usize) -> Self {
        Lienzo { w, h, px: vec![0; w * h], copia: Vec::new() }
    }

    pub fn limpiar(&mut self) {
        self.px.fill(0);
    }

    /// Guarda el fotograma tal cual: de ahí salen las franjas, los fantasmas y la tele que se apaga
    pub fn fotografiar(&mut self) {
        if self.copia.len() != self.px.len() {
            self.copia = vec![0; self.px.len()];
        }
        self.copia.copy_from_slice(&self.px);
    }

    /// Franjas de la foto corridas de lado: (y, alto, dx). Lo que queda al descubierto se
    /// queda como estaba.
    pub fn franjas(&mut self, franjas: &[(i32, i32, i32)]) {
        let w = self.w;
        for &(y0, alto, dx) in franjas {
            let d = (dx.unsigned_abs() as usize).min(w);
            for y in y0.max(0)..(y0 + alto).min(self.h as i32) {
                let fila = y as usize * w;
                let (origen, destino) = (&self.copia[fila..fila + w], &mut self.px[fila..fila + w]);
                if dx >= 0 {
                    destino[d..].copy_from_slice(&origen[..w - d]);
                } else {
                    destino[..w - d].copy_from_slice(&origen[d..]);
                }
            }
        }
    }

    /// Fantasmas rojo y morado de toda la foto, corridos de lado y sumados
    pub fn fantasmas(&mut self, alfa: f32, dx_rojo: i32, dx_morado: i32) {
        let a = alfa256(alfa) as u32;
        if a == 0 {
            return;
        }
        // tintes (pesos de 0 a 256 por canal) ya con el alfa
        let (t1, t2) = ([256u32, 40, 70].map(|v| (v * a) >> 8), [110u32, 20, 256].map(|v| (v * a) >> 8));
        let w = self.w as i32;
        for y in 0..self.h {
            let (origen, destino) = (&self.copia[y * self.w..(y + 1) * self.w], &mut self.px[y * self.w..(y + 1) * self.w]);
            let toma = |x: i32| if (0..w).contains(&x) { origen[x as usize] } else { 0 };
            for (x, o) in destino.iter_mut().enumerate() {
                let (p1, p2) = (toma(x as i32 - dx_rojo), toma(x as i32 - dx_morado));
                if p1 | p2 == 0 {
                    continue;
                }
                let canal = |s: u32, k: usize| ((((p1 >> s) & 255) * t1[k] + ((p2 >> s) & 255) * t2[k]) >> 8) + ((*o >> s) & 255);
                *o = (canal(16, 0).min(255) << 16) | (canal(8, 1).min(255) << 8) | canal(0, 2).min(255);
            }
        }
    }

    /// Rayas de tele: oscurece dos filas de cada cuatro (0 nada, 1 negro)
    pub fn rayas(&mut self, oscuro: f32) {
        let f = (256 - alfa256(oscuro)) as u32;
        for y in (0..self.h).filter(|y| y & 3 < 2) {
            for p in &mut self.px[y * self.w..(y + 1) * self.w] {
                *p = ((((*p & 0xFF00FF) * f) >> 8) & 0xFF00FF) | ((((*p & 0x00FF00) * f) >> 8) & 0x00FF00);
            }
        }
    }

    /// Tele que se apaga: pantalla negra con la foto aplastada en una banda de `alto` filas en
    /// el centro, aclarada hacia `color`
    pub fn aplastar(&mut self, alto: f32, color: Color, brillo: f32) {
        self.px.fill(0);
        let h = (alto as usize).clamp(2, self.h);
        let (y0, c, b) = ((self.h - h) / 2, color_entero(&color), alfa256(brillo));
        for i in 0..h {
            let (o, d) = (i * self.h / h * self.w, (y0 + i) * self.w);
            self.px[d..d + self.w].copy_from_slice(&self.copia[o..o + self.w]);
            if b > 0 {
                for p in &mut self.px[d..d + self.w] {
                    *p = mezclar(*p, &c, b << 8, Mezcla::Suma);
                }
            }
        }
    }

    /// Límites enteros (recortados a la pantalla) de un rectángulo en coma flotante
    fn limites(&self, x: f32, y: f32, w: f32, h: f32) -> Option<(i32, i32, i32, i32)> {
        let x0 = entero_abajo(x).max(0);
        let y0 = entero_abajo(y).max(0);
        let x1 = (ceil(x + w) as i32).min(self.w as i32);
        let y1 = (ceil(y + h) as i32).min(self.h as i32);
        (x0 < x1 && y0 < y1).then_some((x0, y0, x1, y1))
    }

    /// Dibuja el recorte `rec` (x0, y0, x1, y1 del sprite) de una máscara con su esquina en
    /// (x, y) y escalado `esc`.
    #[allow(clippy::too_many_arguments)]
    pub fn mascara(&mut self, s: &Sprite, rec: [i32; 4], x: f32, y: f32, esc: f32, color: Color, alfa: f32, mezcla: Mezcla) {
        let a = alfa256(alfa);
        if a == 0 || esc <= 0.0 {
            return;
        }
        if abs(esc - 1.0) < 0.002 {
            return self.mascara_tal_cual(s, rec, floor(x + 0.5) as i32, floor(y + 0.5) as i32, color, alfa, mezcla);
        }
        let (rw, rh) = ((rec[2] - rec[0]) as f32 * esc, (rec[3] - rec[1]) as f32 * esc);
        let Some((x0, y0, x1, y1)) = self.limites(x, y, rw, rh) else { return };
        let inv = 1.0 / esc;
        let paso = fijo(inv);
        let u0 = fijo((x0 as f32 + 0.5 - x) * inv + rec[0] as f32);
        let mut v = fijo((y0 as f32 + 0.5 - y) * inv + rec[1] as f32);
        let (m, c) = (s.bytes(), color_entero(&color));
        for py in y0..y1 {
            let fila = &mut self.px[py as usize * self.w..(py as usize + 1) * self.w];
            let mut u = u0;
            for p in &mut fila[x0 as usize..x1 as usize] {
                let cob = (muestra(m, s.w, rec, u, v) * a) >> 8;
                if cob > 0 {
                    *p = mezclar(*p, &c, cob, mezcla);
                }
                u += paso;
            }
            v += paso;
        }
    }

    /// Sin escala y en píxel entero: copia directa, sin interpolar (paneles, textos y logos quietos)
    #[allow(clippy::too_many_arguments)]
    fn mascara_tal_cual(&mut self, s: &Sprite, rec: [i32; 4], x: i32, y: i32, color: Color, alfa: f32, mezcla: Mezcla) {
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + rec[2] - rec[0]).min(self.w as i32);
        let y1 = (y + rec[3] - rec[1]).min(self.h as i32);
        let (m, c, a) = (s.bytes(), color_entero(&color), alfa256(alfa));
        for py in y0..y1 {
            let origen = (py - y + rec[1]) as usize * s.w + (rec[0] - x) as usize;
            let fila = py as usize * self.w;
            for px in x0..x1 {
                let b = byte(m, origen.wrapping_add(px as usize));
                if b != 0 {
                    let d = &mut self.px[fila + px as usize];
                    *d = mezclar(*d, &c, b * a, mezcla);
                }
            }
        }
    }

    /// Texto (máscara) en su ancla, sin escalar y en píxel entero para que salga nítido
    pub fn texto(&mut self, s: &Sprite, x: f32, y: f32, color: Color, alfa: f32) {
        self.mascara_tal_cual(s, s.entero(), floor(x - s.cx + 0.5) as i32, floor(y - s.cy + 0.5) as i32, color, alfa, Mezcla::Normal);
    }

    /// Suma una capa pequeña de color (0x00RRGGBB por celda; el fuego de Arch) estirada a w×h
    /// con escala bilineal. Donde las cuatro celdas vecinas son negras no hace nada, que es
    /// casi toda la capa. Los canales van de dos en dos: 0x00RR00BB y 0x0000GG00.
    #[allow(clippy::too_many_arguments)]
    pub fn capa_suma(&mut self, c: &[u32], cw: usize, ch: usize, x: f32, y: f32, w: f32, h: f32) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let Some((x0, y0, x1, y1)) = self.limites(x, y, w, h) else { return };
        let (kx, ky) = (cw as f32 / w, ch as f32 / h);
        let (pu, pv) = (fijo(kx), fijo(ky));
        let u0 = fijo((x0 as f32 + 0.5 - x) * kx) - UNO / 2;
        let mut v = fijo((y0 as f32 + 0.5 - y) * ky) - UNO / 2;
        let (sw, sh) = (cw as i32, ch as i32);
        let mezcla = |p: u32, q: u32, t: u32| {
            let rb = (((p & 0xFF00FF) * (256 - t) + (q & 0xFF00FF) * t) >> 8) & 0xFF00FF;
            let g = (((p & 0x00FF00) * (256 - t) + (q & 0x00FF00) * t) >> 8) & 0x00FF00;
            rb | g
        };
        for py in y0..y1 {
            let (iv, ty) = (v >> 16, ((v >> 8) & 255) as u32);
            let (ya, yb) = (iv.clamp(0, sh - 1) as usize * cw, (iv + 1).clamp(0, sh - 1) as usize * cw);
            let fila = &mut self.px[py as usize * self.w..(py as usize + 1) * self.w];
            let mut u = u0;
            for p in &mut fila[x0 as usize..x1 as usize] {
                let (iu, tx) = (u >> 16, ((u >> 8) & 255) as u32);
                let (xa, xb) = (iu.clamp(0, sw - 1) as usize, (iu + 1).clamp(0, sw - 1) as usize);
                let (a, b, d, e) = (c[ya + xa], c[ya + xb], c[yb + xa], c[yb + xb]);
                if a | b | d | e != 0 {
                    let s = mezcla(mezcla(a, b, tx), mezcla(d, e, tx), ty);
                    let o = *p;
                    let r = (((o >> 16) & 255) + ((s >> 16) & 255)).min(255);
                    let g = (((o >> 8) & 255) + ((s >> 8) & 255)).min(255);
                    let b = ((o & 255) + (s & 255)).min(255);
                    *p = (r << 16) | (g << 8) | b;
                }
                u += pu;
            }
            v += pv;
        }
    }

    /// Rectángulo de color con los bordes suavizados (rayas bajo los nombres, cristales de Windows)
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color, alfa: f32) {
        let a = alfa256(alfa);
        if a == 0 {
            return;
        }
        let Some((x0, y0, x1, y1)) = self.limites(x, y, w, h) else { return };
        // cuánto de cada fila y columna cae dentro, de 0 a 256
        let cubre = |p: i32, ini: f32, fin: f32| alfa256((p as f32 + 1.0).min(fin) - (p as f32).max(ini));
        let cols: Vec<i32> = (x0..x1).map(|px| cubre(px, x, x + w)).collect();
        let c = color_entero(&color);
        for py in y0..y1 {
            let fy = (cubre(py, y, y + h) * a) >> 8;
            let fila = &mut self.px[py as usize * self.w..(py as usize + 1) * self.w];
            for (p, &fx) in fila[x0 as usize..x1 as usize].iter_mut().zip(&cols) {
                *p = mezclar(*p, &c, fx * fy, Mezcla::Normal);
            }
        }
    }

    /// Funde hacia negro: k = 0 nada, k = 1 negro
    pub fn oscurecer(&mut self, k: f32) {
        let f = (256 - alfa256(k)) as u32;
        for p in self.px.iter_mut() {
            *p = ((((*p & 0xFF00FF) * f) >> 8) & 0xFF00FF) | ((((*p & 0x00FF00) * f) >> 8) & 0x00FF00);
        }
    }

    /// Rayas de barrido del dragón de Kali: oscurece una fila de cada cuatro del logo y
    /// suma la franja que baja (centro `barrido` en filas del sprite).
    #[allow(clippy::too_many_arguments)]
    pub fn lineas(&mut self, s: &Sprite, x: f32, y: f32, esc: f32, oscuro: f32, barrido: f32, color: Color, alfa_barrido: f32) {
        let o = alfa256(oscuro);
        if o == 0 {
            return;
        }
        let Some((x0, y0, x1, y1)) = self.limites(x, y, s.w as f32 * esc, s.h as f32 * esc) else { return };
        let (m, rec, c) = (s.bytes(), s.entero(), color_entero(&color));
        let inv = 1.0 / esc;
        let paso = fijo(inv);
        let u0 = fijo((x0 as f32 + 0.5 - x) * inv);
        let mut v = fijo((y0 as f32 + 0.5 - y) * inv);
        let oscuro = (141 * o) >> 8; // 0,55 del negro
        let (centro, fuerza) = (fijo(barrido), alfa256(0.95 * alfa_barrido));
        let (cuatro, raya) = (4 * UNO, fijo(1.6));
        for py in y0..y1 {
            let en_raya = v.rem_euclid(cuatro) < raya;
            // la franja se apaga en 18 filas a cada lado de su centro
            let franja = (((UNO - (v - centro).abs() / 18).max(0) as i64 * fuerza as i64) >> 16) as i32;
            if en_raya || franja > 0 {
                let fila = &mut self.px[py as usize * self.w..(py as usize + 1) * self.w];
                let mut u = u0;
                for p in &mut fila[x0 as usize..x1 as usize] {
                    let a = muestra(m, s.w, rec, u, v);
                    if a > 0 {
                        if en_raya {
                            *p = mezclar(*p, &[0; 3], (a * oscuro) >> 8, Mezcla::Normal);
                        }
                        if franja > 0 {
                            *p = mezclar(*p, &c, (a * franja) >> 8, Mezcla::Suma);
                        }
                    }
                    u += paso;
                }
            }
            v += paso;
        }
    }

    /// Pone de fondo la capa de un cuarto de resolución, ampliada ×4 (bilineal: con ×4 exacto
    /// los pesos son siempre 3/8-5/8 o 1/8-7/8) y con un tramado ordenado para que los
    /// degradados oscuros no hagan escalones. Los tres canales van juntos en un u64 (carriles
    /// de 21 bits), así cada mezcla es una sola multiplicación.
    pub fn ampliar(&mut self, c: &mut Capa) {
        // tramado de Bayer 4×4 en 1/256 de nivel
        const TRAMA: [u64; 16] = [8, 136, 40, 168, 200, 72, 232, 104, 56, 184, 24, 152, 248, 120, 216, 88];
        // fase de cada fila de 4: (vecino de antes, peso de antes, peso propio) en octavos
        const FASES: [(bool, u64, u64); 4] = [(true, 3, 5), (true, 1, 7), (false, 7, 1), (false, 5, 3)];
        const UNOS: u64 = 1 | (1 << 21) | (1 << 42);
        const MASCARA: u64 = ((1 << 18) - 1) * UNOS;
        debug_assert!(self.w == c.w * 4 && self.h == c.h * 4);
        for (d, p) in c.juntos.iter_mut().zip(&c.px) {
            *d = (p[0].min(65280) as u64) << 42 | (p[1].min(65280) as u64) << 21 | p[2].min(65280) as u64;
        }
        let sacar = |x: u64| (((x >> 53) & 255) << 16 | ((x >> 32) & 255) << 8 | ((x >> 11) & 255)) as u32;
        let mut vertical = vec![0u64; c.w];
        for y in 0..self.h {
            let (j, (antes, pa, pb)) = (y / 4, FASES[y & 3]);
            let (ya, yb) = if antes { (j.saturating_sub(1), j) } else { (j, (j + 1).min(c.h - 1)) };
            let (fa, fb) = (&c.juntos[ya * c.w..(ya + 1) * c.w], &c.juntos[yb * c.w..(yb + 1) * c.w]);
            for ((v, &a), &b) in vertical.iter_mut().zip(fa).zip(fb) {
                *v = ((a * pa + b * pb) >> 3) & MASCARA;
            }
            let t = &TRAMA[(y & 3) * 4..(y & 3) * 4 + 4];
            let (t0, t1, t2, t3) = (t[0] * 8 * UNOS, t[1] * 8 * UNOS, t[2] * 8 * UNOS, t[3] * 8 * UNOS);
            let fila = &mut self.px[y * self.w..(y + 1) * self.w];
            for (i, s) in fila.chunks_exact_mut(4).enumerate() {
                let (izq, cen, der) = (vertical[i.saturating_sub(1)], vertical[i], vertical[(i + 1).min(c.w - 1)]);
                s[0] = sacar(izq * 3 + cen * 5 + t0);
                s[1] = sacar(izq + cen * 7 + t1);
                s[2] = sacar(cen * 7 + der + t2);
                s[3] = sacar(cen * 5 + der * 3 + t3);
            }
        }
    }

    /// Quema la pantalla con una capa de un cuarto de resolución (el papel de Arch): por celda,
    /// 0xNNRRGGBB, cuánto negro (NN) y el color que se suma después. Se amplía ×4 como
    /// `ampliar` (bilineal, pesos en octavos) con los cuatro valores juntos en un u64 (carriles
    /// de 16 bits). Donde las celdas vecinas son todas 0 (lo que aún no arde) no toca nada.
    pub fn quemar(&mut self, c: &[u32], cw: usize, ch: usize) {
        const FASES: [(bool, u64, u64); 4] = [(true, 3, 5), (true, 1, 7), (false, 7, 1), (false, 5, 3)];
        // (izquierda, centro, derecha) de cada columna de 4
        const PESOS: [(u64, u64, u64); 4] = [(3, 5, 0), (1, 7, 0), (0, 7, 1), (0, 5, 3)];
        debug_assert!(self.w == cw * 4 && self.h == ch * 4);
        let abrir = |p: u32| ((p >> 24) as u64) << 48 | (((p >> 16) & 255) as u64) << 32 | (((p >> 8) & 255) as u64) << 16 | (p & 255) as u64;
        let mut vertical = vec![0u64; cw];
        for y in 0..self.h {
            let (j, (antes, pa, pb)) = (y / 4, FASES[y & 3]);
            let (ya, yb) = if antes { (j.saturating_sub(1), j) } else { (j, (j + 1).min(ch - 1)) };
            let (fa, fb) = (&c[ya * cw..(ya + 1) * cw], &c[yb * cw..(yb + 1) * cw]);
            // ≤ 255 × 8 por carril
            for ((v, &a), &b) in vertical.iter_mut().zip(fa).zip(fb) {
                *v = abrir(a) * pa + abrir(b) * pb;
            }
            let fila = &mut self.px[y * self.w..(y + 1) * self.w];
            for (i, s) in fila.chunks_exact_mut(4).enumerate() {
                let (izq, cen, der) = (vertical[i.saturating_sub(1)], vertical[i], vertical[(i + 1).min(cw - 1)]);
                if izq | cen | der == 0 {
                    continue;
                }
                for (p, &(a, b, d)) in s.iter_mut().zip(&PESOS) {
                    // ≤ 255 × 64 por carril: cada valor sale con >> 6
                    let m = izq * a + cen * b + der * d;
                    let n = (m >> 54) as u32;
                    let f = 256 - (n + (n >> 7));
                    let o = ((((*p & 0xFF00FF) * f) >> 8) & 0xFF00FF) | ((((*p & 0x00FF00) * f) >> 8) & 0x00FF00);
                    let r = (((o >> 16) & 255) + ((m >> 38) & 1023) as u32).min(255);
                    let g = (((o >> 8) & 255) + ((m >> 22) & 1023) as u32).min(255);
                    let b = ((o & 255) + ((m >> 6) & 1023) as u32).min(255);
                    *p = (r << 16) | (g << 8) | b;
                }
            }
        }
    }
}

/// Capa del fondo (humo y auras) a un cuarto de resolución, en enteros ×256
pub struct Capa {
    pub w: usize,
    pub h: usize,
    pub px: Vec<[u32; 3]>,
    /// Los tres canales en un u64, para ampliar
    juntos: Vec<u64>,
}

impl Capa {
    pub fn new(w: usize, h: usize) -> Self {
        Capa { w, h, px: vec![[0; 3]; w * h], juntos: vec![0; w * h] }
    }

    pub fn limpiar(&mut self) {
        self.px.fill([0; 3]);
    }

    /// Suma una máscara redonda (humo, auras) centrada en (cx, cy), escalada y girada
    #[allow(clippy::too_many_arguments)]
    pub fn girada(&mut self, s: &Sprite, cx: f32, cy: f32, esc: f32, giro: f32, color: Color, alfa: f32) {
        let a = alfa256(alfa);
        if a == 0 || esc <= 0.0 {
            return;
        }
        let (sn, cs, inv) = (sin(giro), cos(giro), 1.0 / esc);
        // fuera del círculo inscrito del sprite no hay nada
        let radio = (s.w.min(s.h) as f32 / 2.0 - 1.0) * esc;
        let y0 = entero_abajo(cy - radio).max(0);
        let y1 = (ceil(cy + radio) as i32).min(self.h as i32);
        if y0 >= y1 {
            return;
        }
        // (u, v) del sprite, ya con el medio píxel restado, en el píxel (0, y0) y sus pasos
        let (mw, mh) = (s.w as f32 / 2.0 - 0.5, s.h as f32 / 2.0 - 0.5);
        let (dx, dy) = (0.5 - cx, y0 as f32 + 0.5 - cy);
        let (mut u_fila, mut v_fila) = (fijo((dx * cs + dy * sn) * inv + mw), fijo((dy * cs - dx * sn) * inv + mh));
        let (ux, vx, uy, vy) = (fijo(cs * inv), fijo(-sn * inv), fijo(sn * inv), fijo(cs * inv));
        // círculo en 1/256 de píxel
        let (cx8, r8) = ((cx * 256.0) as i64, (radio * 256.0) as i64);
        let mut dy8 = ((y0 as f32 + 0.5 - cy) * 256.0) as i64;
        let k = color_entero(&color).map(|v| (v * a) as u32); // ≤ 65280
        let (m, sw, sh) = (s.bytes(), s.w as i32, s.h as i32);
        for py in y0..y1 {
            let dentro = r8 * r8 - dy8 * dy8;
            if dentro > 0 {
                let medio = raiz(dentro as u64) as i64;
                let x0 = ((cx8 - medio) >> 8).max(0) as i32;
                let x1 = (((cx8 + medio) >> 8) + 1).min(self.w as i64) as i32;
                if x0 < x1 {
                    let (mut u, mut v) = (u_fila + x0 * ux, v_fila + x0 * vx);
                    let fila = &mut self.px[py as usize * self.w..(py as usize + 1) * self.w];
                    for p in &mut fila[x0 as usize..x1 as usize] {
                        let (iu, iv) = (u >> 16, v >> 16);
                        if iu >= 0 && iv >= 0 && iu + 1 < sw && iv + 1 < sh {
                            let (tx, ty) = ((u >> 8) & 255, (v >> 8) & 255);
                            let i = (iv * sw + iu) as usize;
                            let arriba = byte(m, i) * (256 - tx) + byte(m, i + 1) * tx;
                            let abajo = byte(m, i + s.w) * (256 - tx) + byte(m, i + s.w + 1) * tx;
                            let b = (((arriba * (256 - ty) + abajo * ty) >> 8) >> 4) as u32; // 0..4096
                            if b > 0 {
                                p[0] += (k[0] * b) >> 12;
                                p[1] += (k[1] * b) >> 12;
                                p[2] += (k[2] * b) >> 12;
                            }
                        }
                        u += ux;
                        v += vx;
                    }
                }
            }
            u_fila += uy;
            v_fila += vy;
            dy8 += 256;
        }
    }
}
