//! La escena del menú: lo mismo que maqueta/menu.js con el diseño elegido (fondo de humo
//! que deriva, cuadros de Windows, fuego nublado de Arch, glitch de Kali y la animación propia
//! de cada sistema al lanzarlo), sin la pista de teclas.

use alloc::vec::Vec;

use crate::grafico::{raiz, Capa, Color, Lienzo, Mezcla, Sprite};
use crate::mates::{aprox, ceil, cos, lim, pow, resto, salida, sin, suave, Azar, PI};
use crate::recursos as r;

pub const W: f32 = 1920.0;
pub const H: f32 = 1080.0;
const Y: f32 = 455.0;
const SEP: f32 = 470.0;
/// La capa del fondo va a un cuarto de resolución
pub const FONDO: f32 = 0.25;
/// Celdas del papel que arde al lanzar Arch: las de la capa de ¼
const PW: usize = 480;
const PH: usize = 270;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sis {
    Windows = 0,
    Arch = 1,
    Kali = 2,
}
const SISES: [Sis; 3] = [Sis::Windows, Sis::Arch, Sis::Kali];
/// De izquierda a derecha
const ORDEN: [Sis; 3] = [Sis::Kali, Sis::Windows, Sis::Arch];

const LOGO: [Color; 3] = [[14.0, 128.0, 224.0], [155.0, 92.0, 255.0], [192.0, 24.0, 47.0]];
const AURA: [Color; 3] = [[30.0, 110.0, 255.0], [139.0, 61.0, 255.0], [226.0, 32.0, 62.0]];
const SPR_LOGO: [Sprite; 3] = [r::LOGO_WINDOWS, r::LOGO_ARCH, r::LOGO_KALI];
const SPR_AURA: [Sprite; 3] = [r::AURA_WINDOWS, r::AURA_ARCH, r::AURA_KALI];
const SPR_NOMBRE: [Sprite; 3] = [r::NOMBRE_WINDOWS, r::NOMBRE_ARCH, r::NOMBRE_KALI];
const SPR_HUMO: [Sprite; 3] = [r::HUMO_0, r::HUMO_1, r::HUMO_2];
const SPR_CUENTA: [Sprite; 10] = [
    r::CUENTA_1, r::CUENTA_2, r::CUENTA_3, r::CUENTA_4, r::CUENTA_5,
    r::CUENTA_6, r::CUENTA_7, r::CUENTA_8, r::CUENTA_9, r::CUENTA_10,
];
const SPR_DIGITO: [Sprite; 10] = [
    r::DIGITO_0, r::DIGITO_1, r::DIGITO_2, r::DIGITO_3, r::DIGITO_4,
    r::DIGITO_5, r::DIGITO_6, r::DIGITO_7, r::DIGITO_8, r::DIGITO_9,
];
/// Sombras del fondo: rojo sangre y púrpura
const SOMBRA: [Color; 2] = [[150.0, 8.0, 30.0], [92.0, 10.0, 196.0]];
const CLARO: Color = [236.0, 232.0, 242.0];
const BLANCO: Color = [255.0, 255.0, 255.0];

/// Lo que se abre al terminar la animación de arranque
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Destino {
    Sistema(Sis),
    Rescate,
    Refind,
    Bios,
    Reiniciar,
    Apagar,
}

struct Opcion {
    destino: Destino,
    icono: Sprite,
    nombre: Sprite,
    detalle: Option<Sprite>,
}
const OPCIONES: [Opcion; 5] = [
    Opcion { destino: Destino::Rescate, icono: r::OPCION_0_ICONO, nombre: r::OPCION_0_NOMBRE, detalle: None },
    Opcion { destino: Destino::Refind, icono: r::OPCION_1_ICONO, nombre: r::OPCION_1_NOMBRE, detalle: None },
    Opcion { destino: Destino::Bios, icono: r::OPCION_2_ICONO, nombre: r::OPCION_2_NOMBRE, detalle: None },
    Opcion { destino: Destino::Reiniciar, icono: r::OPCION_3_ICONO, nombre: r::OPCION_3_NOMBRE, detalle: None },
    Opcion { destino: Destino::Apagar, icono: r::OPCION_4_ICONO, nombre: r::OPCION_4_NOMBRE, detalle: None },
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tecla {
    Izquierda,
    Derecha,
    Arriba,
    Abajo,
    Intro,
    Esc,
    Fps,
}

/// Cómo está la memoria de vídeo (lo dice main; se ve con la tecla F)
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Video {
    SinCache,
    Wc,
    Fallo,
}

struct Arranque {
    destino: Destino,
    t: f32,
    lanzado: bool,
}
impl Arranque {
    /// Los sistemas hacen su animación; las opciones de Esc solo funden a negro
    fn rapido(&self) -> bool {
        !matches!(self.destino, Destino::Sistema(_))
    }
    /// Lo que dura su animación, hasta abrirlo
    fn fin(&self) -> f32 {
        match self.destino {
            Destino::Sistema(Sis::Kali) => 1.35,
            Destino::Sistema(Sis::Windows) => 1.45,
            Destino::Sistema(Sis::Arch) => 1.8,
            _ => 0.45,
        }
    }
    fn negro(&self) -> f32 {
        match self.destino {
            // la tele ya se apaga sola
            Destino::Sistema(Sis::Kali) => lim((self.t - 1.3) / 0.05),
            Destino::Sistema(_) => lim((self.t - self.fin() + 0.35) / 0.35),
            _ => lim(self.t / 0.4),
        }
    }
}

/// Lo que cambia en cada fotograma del glitch de toda la pantalla al lanzar Kali
struct GlitchPantalla {
    /// (y, alto, dx)
    franjas: Vec<(i32, i32, i32)>,
    /// (y, alto, rojo, alfa)
    destello: Option<(f32, f32, bool, f32)>,
}

/// Una bocanada de humo que da vueltas lentas alrededor de un punto del borde
struct Humo {
    x: f32,
    y: f32,
    color: usize,
    spr: usize,
    ax: f32,
    ay: f32,
    fx: f32,
    fy: f32,
    fase: f32,
    giro: f32,
    g0: f32,
    esc: f32,
    alfa: f32,
}

#[derive(Clone, Copy)]
struct Corte {
    y: i32,
    h: i32,
    dx: f32,
}

struct Glitch {
    hasta: f32,
    proximo: f32,
    refresco: f32,
    activo: bool,
    cortes: Vec<Corte>,
    fx: f32,
    aura: f32,
}

/// Posición, escala y alfa de un sistema en este fotograma
struct Estado {
    x: f32,
    y: f32,
    esc: f32,
    alfa: f32,
    a: f32,
    vis: f32,
    fx: f32,
}

pub struct Escena {
    t: f32,
    reloj: f32,
    espera: f32,
    elegido: Sis,
    anim: [f32; 3],
    peso: [f32; 3],
    aura_x: f32,
    cuenta: f32,
    cuenta_activa: bool,
    cuenta_alfa: f32,
    menu_abierto: bool,
    menu_m: f32,
    menu_i: usize,
    arranque: Option<Arranque>,
    aviso: Option<(Sprite, f32)>,
    glitch: Glitch,
    /// Capa del fuego de Arch (celdas de NUBLADO_NUBE, 0x00RRGGBB); vacía si no se ve
    fuego: Vec<u32>,
    fuego_visible: bool,
    /// 0..1: el fuego de Arch prende poco a poco al elegirlo
    encendido: f32,
    /// El papel que arde al lanzar Arch (PW×PH, 0xNNRRGGBB: negro y color que se suma)
    papel: Vec<u32>,
    /// Distancia de cada celda a la A con el borde irregular, en 1/16 de celda
    papel_d: Vec<i16>,
    /// Calor que sube por cada columna del papel (en 1/256)
    calor: Vec<i32>,
    video: Video,
    pantalla: GlitchPantalla,
    humo: Vec<Humo>,
    azar: Azar,
    fps: bool,
    /// fps y milisegundos de dibujar, pasar a pantalla y leer el teclado (los mide main)
    medidas: [u32; 4],
}

const fn pos(s: Sis) -> usize {
    match s {
        Sis::Kali => 0,
        Sis::Windows => 1,
        Sis::Arch => 2,
    }
}
fn pos_x(s: Sis) -> f32 {
    W / 2.0 + (pos(s) as f32 - 1.0) * SEP
}

/// Distancia de cada celda del papel a la A, en 1/16 de celda, más lo irregular del borde:
/// fijo (no se mueve), como las fibras de un papel
fn distancias_papel() -> Vec<i16> {
    let (ruido, giro) = (r::NUBLADO_RUIDO.bytes(), r::NUBLADO_GIRO.bytes());
    let en = |m: &[u8], fila: i32, col: i32| m[(((fila & 255) << 8) | (col & 255)) as usize] as i32;
    let (cx, cy) = ((pos_x(Sis::Arch) * FONDO * 16.0) as i64, (Y * FONDO * 16.0) as i64);
    let mut d = Vec::with_capacity(PW * PH);
    for y in 0..PH as i32 {
        for x in 0..PW as i32 {
            let (dx, dy) = (x as i64 * 16 - cx, y as i64 * 16 - cy);
            // (giro − 128) × 0,22 + (ruido − 128) × 0,07 celdas
            let irregular = ((en(giro, y * 2, x * 2) - 128) * 352 + (en(ruido, y * 5 + 31, x * 5) - 128) * 112) / 100;
            d.push((raiz((dx * dx + dy * dy) as u64) as i32 + irregular) as i16);
        }
    }
    d
}

impl Escena {
    pub fn new(semilla: u64, espera: f32) -> Self {
        let mut azar = Azar::new(semilla);
        let puntos = [(170.0, 150.0, 0), (1750.0, 170.0, 1), (110.0, 900.0, 1), (1810.0, 930.0, 0), (960.0, 1130.0, 0), (960.0, -70.0, 1), (560.0, 1060.0, 1), (1380.0, 30.0, 0)];
        let humo = puntos
            .iter()
            .enumerate()
            .map(|(i, &(x, y, color))| Humo {
                x,
                y,
                color,
                spr: i % 3,
                ax: azar.entre(140.0, 300.0),
                ay: azar.entre(60.0, 170.0),
                fx: azar.entre(0.012, 0.028),
                fy: azar.entre(0.01, 0.024),
                fase: azar.entre(0.0, 6.28),
                giro: azar.entre(-0.035, 0.035),
                g0: azar.entre(0.0, 6.28),
                esc: azar.entre(2.3, 3.2),
                alfa: azar.entre(0.45, 0.7),
            })
            .collect();
        let mut e = Escena {
            t: 0.0,
            reloj: 0.0,
            espera,
            elegido: Sis::Windows,
            anim: [0.0; 3],
            peso: [0.0; 3],
            aura_x: 0.0,
            cuenta: espera,
            cuenta_activa: true,
            cuenta_alfa: 1.0,
            menu_abierto: false,
            menu_m: 0.0,
            menu_i: 0,
            arranque: None,
            aviso: None,
            glitch: Glitch { hasta: 0.0, proximo: 1.5, refresco: 0.0, activo: false, cortes: Vec::new(), fx: 8.0, aura: 1.0 },
            fuego: alloc::vec![0; r::NUBLADO_NUBE.w * r::NUBLADO_NUBE.h],
            fuego_visible: false,
            encendido: 0.0,
            papel: alloc::vec![0; PW * PH],
            papel_d: distancias_papel(),
            calor: alloc::vec![0; PW],
            video: Video::SinCache,
            pantalla: GlitchPantalla { franjas: Vec::new(), destello: None },
            humo,
            azar,
            fps: false,
            medidas: [0; 4],
        };
        e.reiniciar();
        e
    }

    fn reiniciar(&mut self) {
        self.reloj = 0.0;
        self.elegido = Sis::Windows;
        self.cuenta = self.espera;
        self.cuenta_activa = true;
        self.cuenta_alfa = 1.0;
        self.arranque = None;
        self.menu_abierto = false;
        self.menu_m = 0.0;
        self.menu_i = 0;
        for s in SISES {
            let v = if s == Sis::Windows { 1.0 } else { 0.0 };
            self.anim[s as usize] = v;
            self.peso[s as usize] = v;
        }
        self.aura_x = pos_x(Sis::Windows);
        self.encendido = 0.0;
    }

    /// No se pudo abrir el destino: vuelve el menú (sin cuenta atrás) con el aviso
    pub fn fallo(&mut self, d: Destino) {
        let elegido = self.elegido;
        self.reiniciar();
        self.elegido = elegido;
        self.cuenta_activa = false;
        self.cuenta_alfa = 0.0;
        let aviso = match d {
            Destino::Sistema(Sis::Windows) => r::ERROR_WINDOWS,
            Destino::Sistema(Sis::Arch) => r::ERROR_ARCH,
            Destino::Sistema(Sis::Kali) => r::ERROR_KALI,
            Destino::Rescate => r::ERROR_RESCATE,
            Destino::Refind => r::ERROR_REFIND,
            _ => r::ERROR_BIOS,
        };
        self.aviso = Some((aviso, 0.0));
    }

    /// Lo que se abrió terminó y devolvió el control: vuelta al menú
    pub fn volver(&mut self) {
        self.reiniciar();
        self.cuenta_activa = false;
        self.cuenta_alfa = 0.0;
    }

    pub fn medir(&mut self, medidas: [u32; 4]) {
        self.medidas = medidas;
    }

    pub fn video(&mut self, v: Video) {
        self.video = v;
    }

    pub fn tecla(&mut self, k: Tecla) {
        if k == Tecla::Fps {
            self.fps = !self.fps;
            return;
        }
        if self.arranque.is_some() || self.reloj < 0.3 {
            return;
        }
        self.cuenta_activa = false;
        if self.menu_abierto {
            match k {
                Tecla::Arriba => self.menu_i = (self.menu_i + OPCIONES.len() - 1) % OPCIONES.len(),
                Tecla::Abajo => self.menu_i = (self.menu_i + 1) % OPCIONES.len(),
                Tecla::Intro => self.arrancar(OPCIONES[self.menu_i].destino),
                Tecla::Esc => self.menu_abierto = false,
                _ => {}
            }
            return;
        }
        let i = pos(self.elegido);
        match k {
            Tecla::Izquierda => self.elegido = ORDEN[i.saturating_sub(1)],
            Tecla::Derecha => self.elegido = ORDEN[(i + 1).min(2)],
            Tecla::Intro => self.arrancar(Destino::Sistema(self.elegido)),
            Tecla::Esc => {
                self.menu_abierto = true;
                self.menu_i = 0;
            }
            _ => {}
        }
    }

    fn arrancar(&mut self, d: Destino) {
        self.arranque = Some(Arranque { destino: d, t: 0.0, lanzado: false });
        self.cuenta_activa = false;
        if matches!(d, Destino::Sistema(_)) {
            self.menu_abierto = false;
        }
    }

    /// El sistema que se está lanzando y cuánto lleva
    fn lanzando(&self) -> Option<(Sis, f32)> {
        match self.arranque.as_ref()? {
            Arranque { destino: Destino::Sistema(s), t, .. } => Some((*s, *t)),
            _ => None,
        }
    }

    fn estado(&self, s: Sis) -> Estado {
        let a = self.anim[s as usize];
        let vis = salida((self.reloj - 0.1 - pos(s) as f32 * 0.09) / 0.6);
        let (mut esc, mut alfa, mut fx) = (0.74 + 0.26 * a, (0.42 + 0.58 * a) * vis, a * vis);
        if let Some(arr) = &self.arranque {
            if arr.destino == Destino::Sistema(s) {
                match s {
                    // lo pintan sus cristales al abrirse
                    Sis::Windows => (alfa, fx) = (0.0, 0.0),
                    Sis::Arch => esc *= 1.0 + 0.2 * suave(arr.t / 0.9),
                    Sis::Kali => {}
                }
            } else if !arr.rapido() {
                let k = 1.0 - lim(arr.t / 0.35);
                alfa *= k;
                fx *= k;
            }
        }
        Estado { x: pos_x(s), y: Y + (1.0 - vis) * 26.0, esc, alfa, a, vis, fx }
    }

    fn intensidad_aura(&self, s: Sis) -> f32 {
        if s == Sis::Kali && self.glitch.activo {
            self.glitch.aura
        } else {
            0.8 + 0.2 * sin(self.t * 2.0 * PI / 3.4)
        }
    }

    /// Fuego nublado de Arch (pintarNublado de la maqueta): llamas cortas pegadas al borde y una
    /// nube que sube despacio y se enrosca, en una capa de celdas de 2 px. Por celda solo hay
    /// búsquedas en las texturas de ruido, cuentas con enteros y las paletas. Se enciende poco a
    /// poco: prende junto al borde (donde la envolvente es alta) y el frente baja con ruido hasta
    /// dejarla entera; la nube sale cuando ya hay llamas.
    fn nublado(&mut self) {
        let fx = (lim(self.estado(Sis::Arch).fx) * 256.0) as i32;
        let (enc, enc_nube) = (suave(self.encendido), suave((self.encendido - 0.35) / 0.65));
        self.fuego_visible = fx > 0 && enc > 0.003;
        if !self.fuego_visible {
            return;
        }
        let (umbral_c, umbral_n) = (((1.0 - enc) * 320.0) as i32, ((1.0 - enc_nube) * 320.0) as i32);
        let t = self.t;
        let (sube, lento, g1, g2) = ((t * 200.0) as i32, (t * 46.0) as i32, (t * 16.0) as i32, (t * 11.0) as i32);
        let (ruido, giro) = (r::NUBLADO_RUIDO.bytes(), r::NUBLADO_GIRO.bytes());
        let (cerca, nube) = (r::NUBLADO_CERCA.bytes(), r::NUBLADO_NUBE.bytes());
        let (pf, ph) = (r::PALETA_FUEGO.bytes(), r::PALETA_HUMO.bytes());
        let (nw, nh) = (r::NUBLADO_NUBE.w as i32, r::NUBLADO_NUBE.h as i32);
        // las texturas se repiten cada 256
        let en = |m: &[u8], fila: i32, col: i32| m[(((fila & 255) << 8) | (col & 255)) as usize] as i32;
        for y in 0..nh {
            for x in 0..nw {
                // dos giros encadenados: la nube se enrosca
                let w1 = en(giro, y + g1, x) - 128;
                let w2 = en(giro, x + 97 + g2, y + (w1 >> 3)) - 128;
                let (xs, ys) = ((x + (w1 >> 3)).clamp(0, nw - 1), (y + (w2 >> 4)).clamp(0, nh - 1));
                let (ec, enb, n2) = (cerca[(y * nw + x) as usize] as i32, nube[(ys * nw + xs) as usize] as i32, en(ruido, y * 2 + sube, x * 4));
                // cuánto ha prendido cada celda (0..256): el frente se suaviza en 50 y 80 de envolvente
                let kc = (((ec - umbral_c + ((n2 - 128) >> 1)) * 1311) >> 8).min(256);
                let kn = (((enb - umbral_n) * 819) >> 8).min(256);
                let c = if kc > 0 { (((ec * fx) >> 8) * kc) >> 8 } else { 0 };
                let n = if kn > 0 { (((enb * fx) >> 8) * kn) >> 8 } else { 0 };
                // humo = nube × ruido × 1,9 / 255 − 30; fuego = cerca × 1,5 − (255 − ruido) × 0,95
                let humo = if n > 3 { ((n * en(ruido, ys + lento, xs * 2) * 488) >> 16) - 30 } else { 0 };
                let fuego = if c > 3 { ((c * 3) >> 1) - (((255 - n2) * 243) >> 8) } else { 0 };
                let (a, b) = (fuego.clamp(0, 255) as usize * 3, humo.clamp(0, 255) as usize * 3);
                let canal = |k: usize| (pf[a + k] as u32 + ph[b + k] as u32).min(255);
                self.fuego[(y * nw + x) as usize] = (canal(0) << 16) | (canal(1) << 8) | canal(2);
            }
        }
    }

    /// Avanza `dt` segundos; devuelve el destino cuando su animación ha terminado
    pub fn actualizar(&mut self, dt: f32) -> Option<Destino> {
        self.t += dt;
        self.reloj += dt;
        for s in SISES {
            let obj = if s == self.elegido { 1.0 } else { 0.0 };
            self.anim[s as usize] = aprox(self.anim[s as usize], obj, 11.0, dt);
            self.peso[s as usize] = aprox(self.peso[s as usize], obj, 8.0, dt);
        }
        self.aura_x = aprox(self.aura_x, pos_x(self.elegido), 10.0, dt);
        // el fuego de Arch tarda 1,6 s en prender del todo y medio en apagarse
        self.encendido = lim(self.encendido + if self.elegido == Sis::Arch { dt / 1.6 } else { -dt / 0.5 });
        self.menu_m = aprox(self.menu_m, if self.menu_abierto { 1.0 } else { 0.0 }, 14.0, dt);
        self.cuenta_alfa = aprox(self.cuenta_alfa, if self.cuenta_activa { 1.0 } else { 0.0 }, 10.0, dt);
        if self.cuenta_activa && self.reloj > 0.8 && self.arranque.is_none() {
            self.cuenta -= dt;
            if self.cuenta <= 0.0 {
                self.cuenta = 0.0;
                self.arrancar(Destino::Sistema(self.elegido));
            }
        }
        if let Some((_, t)) = &mut self.aviso {
            *t += dt;
            if *t > 4.0 {
                self.aviso = None;
            }
        }

        self.nublado();
        self.glitch_pantalla();

        // glitch de Kali: ráfagas cortas a ratos
        let t = self.t;
        let kali = self.anim[Sis::Kali as usize];
        let g = &mut self.glitch;
        if t >= g.proximo {
            g.hasta = t + self.azar.entre(0.12, 0.3);
            g.proximo = t + self.azar.entre(1.3, 3.4);
        }
        // al lanzar Kali, el logo se glitchea sin parar
        let lanzando_kali = matches!(self.arranque, Some(Arranque { destino: Destino::Sistema(Sis::Kali), t, .. }) if t < 1.0);
        g.activo = (t < g.hasta && kali > 0.3) || lanzando_kali;
        if g.activo && t >= g.refresco {
            g.refresco = t + 0.045;
            g.fx = self.azar.entre(5.0, 13.0);
            g.aura = self.azar.entre(0.35, 1.2);
            g.cortes.clear();
            let mut y = 0;
            while y < r::L as i32 {
                let h = self.azar.entre(6.0, 46.0) as i32;
                let dx = if self.azar.sig() < 0.45 { self.azar.entre(-30.0, 30.0) } else { 0.0 };
                g.cortes.push(Corte { y, h: h.min(r::L as i32 - y), dx });
                y += h;
            }
        }

        let mut listo = None;
        if let Some(arr) = &mut self.arranque {
            arr.t += dt;
            if !arr.lanzado && arr.t >= arr.fin() {
                arr.lanzado = true;
                listo = Some(arr.destino);
            }
        }
        // con el tiempo ya avanzado: es el que usa pintar
        self.papel();
        listo
    }

    pub fn pintar(&self, lz: &mut Lienzo, capa: &mut Capa) {
        let t = self.t;
        let k = FONDO;

        // fondo a un cuarto de resolución: humo que deriva y el aura del elegido
        capa.limpiar();
        for h in &self.humo {
            let x = h.x + sin(t * 2.0 * PI * h.fx + h.fase) * h.ax;
            let y = h.y + cos(t * 2.0 * PI * h.fy + h.fase * 1.3) * h.ay;
            let alfa = h.alfa * (0.75 + 0.25 * sin(t * 0.4 + h.fase));
            // los sprites de humo van a la mitad de tamaño
            capa.girada(&SPR_HUMO[h.spr], x * k, y * k, h.esc * 2.0 * k, h.g0 + t * h.giro, SOMBRA[h.color], alfa);
        }
        let e = self.estado(self.elegido);
        for s in SISES {
            let mut a = self.peso[s as usize] * self.intensidad_aura(s) * e.vis;
            if let Some(arr) = &self.arranque {
                if arr.destino == Destino::Sistema(s) {
                    a *= 1.0 + 0.9 * lim(arr.t / 0.5);
                } else if !arr.rapido() {
                    a *= 1.0 - lim(arr.t / 0.35);
                }
            }
            capa.girada(&SPR_AURA[s as usize], self.aura_x * k, e.y * k, e.esc * k, 0.0, AURA[s as usize], a);
        }
        let lanzando = self.lanzando();
        if let Some((Sis::Windows, ta)) = lanzando {
            self.luz_windows(capa, ta);
        }
        lz.ampliar(capa);

        for s in SISES {
            self.pintar_sistema(lz, s);
        }
        for s in SISES {
            self.pintar_nombre(lz, s);
        }
        self.pintar_menu(lz);
        match lanzando {
            Some((Sis::Windows, ta)) => self.cristales(lz, ta),
            Some((Sis::Arch, _)) => lz.quemar(&self.papel, PW, PH),
            Some((Sis::Kali, ta)) => self.glitch_kali(lz, ta),
            None => {}
        }

        // fundidos a negro: al entrar y al arrancar
        let negro = (1.0 - lim(self.reloj / 0.7)).max(self.arranque.as_ref().map_or(0.0, |a| a.negro()));
        if negro > 0.003 {
            lz.oscurecer(negro);
        }
        if let Some((spr, ta)) = &self.aviso {
            lz.texto(spr, W / 2.0, H - 150.0, CLARO, 0.85 * lim(ta / 0.2) * (1.0 - lim((ta - 3.6) / 0.4)));
        }
        if self.fps {
            // tecla F: lo que tarda cada fotograma, para medir en el portátil
            let etiquetas = [r::MEDIDA_FPS, r::MEDIDA_DIBUJO, r::MEDIDA_PANTALLA, r::MEDIDA_TECLADO];
            for (fila, (etiqueta, &valor)) in etiquetas.iter().zip(&self.medidas).enumerate() {
                let y = 40.0 + fila as f32 * 30.0;
                let mut x = 24.0;
                let mut cifras = Vec::new();
                let mut n = valor;
                loop {
                    cifras.push((n % 10) as usize);
                    n /= 10;
                    if n == 0 {
                        break;
                    }
                }
                for &c in cifras.iter().rev() {
                    let d = &SPR_DIGITO[c];
                    lz.texto(d, x + d.cx, y, BLANCO, 0.85);
                    x += d.w as f32 - 4.0;
                }
                lz.texto(etiqueta, 80.0, y, CLARO, 0.6);
            }
            let video = match self.video {
                Video::SinCache => r::MEDIDA_VIDEO_UC,
                Video::Wc => r::MEDIDA_VIDEO_WC,
                Video::Fallo => r::MEDIDA_VIDEO_FALLO,
            };
            lz.texto(&video, 24.0, 40.0 + 4.0 * 30.0, CLARO, 0.6);
        }
    }

    fn pintar_sistema(&self, lz: &mut Lienzo, s: Sis) {
        let e = self.estado(s);
        let (logo, i) = (&SPR_LOGO[s as usize], s as usize);
        let x0 = e.x - r::L / 2.0 * e.esc;
        let y0 = e.y - r::L / 2.0 * e.esc;
        let t = self.t;
        match s {
            Sis::Windows => {
                lz.mascara(logo, logo.entero(), x0, y0, e.esc, LOGO[i], e.alfa, Mezcla::Normal);
                // los cuatro cuadros laten por turnos
                for (n, c) in r::CUADROS.iter().enumerate() {
                    let f = resto(t - n as f32 * 0.3, 2.4);
                    if f < 0.6 {
                        let rec = [c[0] as i32, c[1] as i32, ceil(c[0] + c[2]) as i32, ceil(c[1] + c[3]) as i32];
                        let (cx, cy) = (x0 + rec[0] as f32 * e.esc, y0 + rec[1] as f32 * e.esc);
                        lz.mascara(logo, rec, cx, cy, e.esc, [159.0, 204.0, 243.0], 0.5 * sin(f / 0.6 * PI) * e.fx, Mezcla::Suma);
                    }
                }
            }
            Sis::Arch => {
                lz.mascara(logo, logo.entero(), x0, y0, e.esc, LOGO[i], e.alfa, Mezcla::Normal);
                if self.fuego_visible {
                    let [zx, zy, zw, zh] = r::NUBLADO_ZONA;
                    let (cw, ch) = (r::NUBLADO_NUBE.w, r::NUBLADO_NUBE.h);
                    lz.capa_suma(&self.fuego, cw, ch, e.x + zx * e.esc, e.y + zy * e.esc, zw * e.esc, zh * e.esc);
                }
            }
            Sis::Kali => {
                let g = &self.glitch;
                if g.activo {
                    // fantasmas rojo y púrpura y el dragón cortado en franjas desplazadas
                    lz.mascara(logo, logo.entero(), x0 + g.fx * e.esc, y0, e.esc, [255.0, 77.0, 104.0], 0.55 * e.alfa, Mezcla::Suma);
                    lz.mascara(logo, logo.entero(), x0 - g.fx * 0.8 * e.esc, y0 + 2.0, e.esc, [122.0, 28.0, 255.0], 0.5 * e.alfa, Mezcla::Suma);
                    for c in &g.cortes {
                        let rec = [0, c.y, r::L as i32, c.y + c.h];
                        lz.mascara(logo, rec, x0 + c.dx * e.esc, y0 + c.y as f32 * e.esc, e.esc, LOGO[i], e.alfa, Mezcla::Normal);
                    }
                } else {
                    lz.mascara(logo, logo.entero(), x0, y0, e.esc, LOGO[i], e.alfa, Mezcla::Normal);
                }
                let f = resto(t, 2.8) / 1.1;
                let (centro, alfa) = if f < 1.0 { (-36.0 + (r::L + 36.0) * f + 18.0, 0.8 * e.fx) } else { (-100.0, 0.0) };
                lz.lineas(logo, x0, y0, e.esc, e.fx, centro, [255.0, 120.0, 135.0], alfa);
            }
        }
    }

    // ------------------------------------------------------------ lanzar cada sistema
    // (lanzarKali, lanzarWindows y lanzarArch de la maqueta)

    /// Kali: lo que cambia en cada fotograma del glitch de toda la pantalla
    fn glitch_pantalla(&mut self) {
        let Some((Sis::Kali, t)) = self.lanzando() else { return };
        let g = suave(t / 0.7);
        let a = &mut self.azar;
        self.pantalla.franjas.clear();
        for _ in 0..4 + (g * 26.0) as usize {
            let (y, alto, dx) = (a.sig() * H, a.entre(3.0, 18.0 + 70.0 * g), a.entre(-1.0, 1.0) * (12.0 + 160.0 * g));
            self.pantalla.franjas.push((y as i32, alto as i32, dx as i32));
        }
        self.pantalla.destello = (a.sig() < 0.2 * g).then(|| (a.sig() * H, a.entre(4.0, 90.0), a.sig() < 0.5, a.entre(0.08, 0.22)));
    }

    /// Kali: toda la pantalla se glitchea cada vez más (franjas corridas, fantasmas rojo y
    /// morado, rayas de tele y destellos) y luego se apaga como una tele vieja: la imagen se
    /// aplasta en una raya y la raya en un punto
    fn glitch_kali(&self, lz: &mut Lienzo, t: f32) {
        lz.fotografiar();
        if t < 1.0 {
            let g = suave(t / 0.7);
            lz.franjas(&self.pantalla.franjas);
            lz.fantasmas(0.55 * g, (6.0 + 26.0 * g) as i32, -(5.0 + 22.0 * g) as i32);
            lz.rayas(0.35 * g);
            if let Some((y, alto, rojo, alfa)) = self.pantalla.destello {
                let color = if rojo { LOGO[Sis::Kali as usize] } else { [232.0, 224.0, 255.0] };
                lz.rect(0.0, y, W, alto, color, alfa);
            }
            return;
        }
        let k = (t - 1.0) / 0.3;
        if k < 0.55 {
            lz.aplastar(H * (1.0 - suave(k / 0.55)), [255.0, 214.0, 220.0], 0.3 * k / 0.55);
        } else {
            lz.limpiar();
            if k < 1.0 {
                let w = (W * (1.0 - suave((k - 0.55) / 0.4))).max(2.0);
                lz.rect((W - w) / 2.0, H / 2.0 - 1.0, w, 2.0, [255.0, 232.0, 236.0], 1.0);
            }
        }
    }

    /// Windows: la luz de detrás de la ventana crece desde el centro (en la capa de ¼)
    fn luz_windows(&self, capa: &mut Capa, t: f32) {
        let (e, p, k) = (self.estado(Sis::Windows), lim(t / 1.25), FONDO);
        let luz = suave(p / 0.8) * (1.0 - lim((t - 1.2) / 0.25));
        let radio = 60.0 + 1500.0 * p * p;
        capa.girada(&r::LUZ, e.x * k, e.y * k, radio * k / 128.0, 0.0, [90.0, 170.0, 255.0], 0.6 * luz);
        capa.girada(&r::LUZ, e.x * k, e.y * k, radio * 0.45 * k / 128.0, 0.0, [235.0, 245.0, 255.0], 0.95 * luz);
    }

    /// Windows: los cuatro cristales se separan hacia sus esquinas mientras te acercas
    fn cristales(&self, lz: &mut Lienzo, t: f32) {
        let (e, p) = (self.estado(Sis::Windows), lim(t / 1.25));
        let (zoom, abre) = (1.0 + 11.0 * pow(p, 2.6), 520.0 * pow(p, 2.2));
        let color = LOGO[Sis::Windows as usize].map(|v| v + (255.0 - v) * 0.25 * p);
        let alfa = 1.0 - lim((p - 0.75) / 0.25);
        let lado = |v: f32| if v < 0.0 { -1.0 } else { 1.0 };
        for c in &r::CUADROS {
            let (cx, cy) = (c[0] + c[2] / 2.0 - r::L / 2.0, c[1] + c[3] / 2.0 - r::L / 2.0);
            let (x, y) = (e.x + cx * zoom + lado(cx) * abre, e.y + cy * zoom + lado(cy) * abre);
            lz.rect(x - c[2] * zoom / 2.0, y - c[3] * zoom / 2.0, c[2] * zoom, c[3] * zoom, color, alfa);
        }
    }

    /// Arch: la pantalla arde como un papel desde la A. Un agujero negro de borde irregular
    /// crece desde el logo; el borde está encendido, de él suben llamas (también por encima de
    /// lo ya quemado) y detrás quedan brasas que se apagan. Las llamas: el calor del borde sube
    /// por cada columna y se va enfriando, y el ruido lo rompe en lenguas altas que suben
    /// deprisa. Mismas texturas y paletas que el fuego nublado; se pinta encima de todo.
    fn papel(&mut self) {
        let Some((Sis::Arch, t)) = self.lanzando() else { return };
        let radio = 470.0 * pow(lim((t - 0.1) / 1.45), 1.4);
        let (r16, prende) = ((radio * 16.0) as i32, radio > 0.0);
        let (sube, sube2, gira) = ((t * 300.0) as i32, (t * 460.0) as i32, (t * 40.0) as i32);
        let (ruido, giro) = (r::NUBLADO_RUIDO.bytes(), r::NUBLADO_GIRO.bytes());
        let (pf, ph) = (r::PALETA_FUEGO.bytes(), r::PALETA_HUMO.bytes());
        let en = |m: &[u8], fila: i32, col: i32| m[(((fila & 255) << 8) | (col & 255)) as usize] as i32;
        self.calor.fill(0);
        // de abajo arriba, para que el calor suba
        for y in (0..PH as i32).rev() {
            for x in 0..PW as i32 {
                let i = y as usize * PW + x as usize;
                // q: dónde está la celda respecto al borde de 7 celdas (q de la maqueta × 112):
                // de 0 a 112 el borde, más lo quemado y menos lo que aún no
                let q = r16 - self.papel_d[i] as i32;
                // fuente de las llamas = lim(1 − |q − 0,4| / 0,8)
                let lejos = (q * 10 - 448).abs();
                let fuente = if prende && lejos < 896 { ((896 - lejos) * 73) >> 8 } else { 0 };
                // negro de lo quemado = lim((q − 0,5) / 0,6)
                let negro = (((q - 56) * 971) >> 8).clamp(0, 255) as u32;
                // brasas justo detrás del borde = lim(1 − (q − 0,8) / 2,5) × 120
                let brasa = if q * 10 > 896 { (120 - (((q * 10 - 896) * 11) >> 8)).max(0) as usize * 3 } else { 0 };
                let mut c = [ph[brasa] as i32, ph[brasa + 1] as i32, ph[brasa + 2] as i32];
                let calor = &mut self.calor[x as usize];
                *calor = (fuente << 8).max((*calor * 238) >> 8);
                if *calor >= 4 << 8 {
                    let w = (en(giro, y * 2 + gira, x * 2) - 128) >> 4;
                    let (n1, n2) = (en(ruido, ((y * 3) >> 2) + sube, (x + w) * 2), en(ruido, y + sube2 + 128, (x + w) * 4 + 64));
                    // v = calor × 1,55 − (255 − ruido) × 1,3
                    let v = ((*calor * 397) >> 16) - (((255 - (n1 * 2 + n2) / 3) * 333) >> 8);
                    if v > 0 {
                        let a = v.min(255) as usize * 3;
                        for (k, canal) in c.iter_mut().enumerate() {
                            *canal += pf[a + k] as i32;
                        }
                    }
                }
                let [cr, cg, cb] = c.map(|v| v.min(255) as u32);
                self.papel[i] = negro << 24 | cr << 16 | cg << 8 | cb;
            }
        }
    }

    fn pintar_nombre(&self, lz: &mut Lienzo, s: Sis) {
        let e = self.estado(s);
        let k = e.vis * self.arranque.as_ref().map_or(1.0, |a| if a.rapido() { 1.0 } else { 1.0 - lim(a.t / 0.3) });
        if k <= 0.003 {
            return;
        }
        lz.texto(&SPR_NOMBRE[s as usize], e.x, e.y + 168.0, CLARO, (0.3 + 0.7 * e.a) * k);
        // raya de su color bajo el elegido; en el de por defecto hace de cuenta atrás
        let cuenta = if s == Sis::Windows { self.cuenta_alfa } else { 0.0 };
        let ancho = 44.0 + (220.0 * self.cuenta / self.espera - 44.0) * cuenta;
        lz.rect(e.x - ancho / 2.0, e.y + 188.0, ancho, 2.0, LOGO[s as usize], e.a.max(cuenta) * k);
        if cuenta > 0.01 {
            let n = (ceil(self.cuenta) as usize).clamp(1, 10);
            lz.texto(&SPR_CUENTA[n - 1], e.x + 1.0, e.y + 224.0, CLARO, 0.5 * cuenta * k);
        }
    }

    fn pintar_menu(&self, lz: &mut Lienzo) {
        let m = self.menu_m;
        if m < 0.01 {
            return;
        }
        lz.oscurecer(0.74 * m);
        let col = LOGO[self.elegido as usize];
        let (alto, ancho) = (78.0, 640.0);
        let n = OPCIONES.len() as f32;
        let x0 = W / 2.0 - ancho / 2.0;
        let y0 = H / 2.0 - n * alto / 2.0 + 30.0 + (1.0 - m) * 18.0;
        let panel = &r::PANEL;
        lz.mascara(panel, panel.entero(), x0 - 26.0, y0 - 96.0, 1.0, [10.0, 8.0, 14.0], 0.9 * m, Mezcla::Normal);
        lz.mascara(&r::PANEL_BORDE, panel.entero(), x0 - 26.0, y0 - 96.0, 1.0, BLANCO, 0.08 * m, Mezcla::Normal);
        lz.texto(&r::MENU_TITULO, W / 2.0, y0 - 44.0, CLARO, 0.5 * m);
        for (i, o) in OPCIONES.iter().enumerate() {
            let y = y0 + i as f32 * alto;
            let sel = i == self.menu_i;
            if sel {
                lz.mascara(&r::RESALTE, r::RESALTE.entero(), x0, y + 4.0, 1.0, col, 0.16 * m, Mezcla::Normal);
                lz.rect(x0, y + 22.0, 4.0, alto - 44.0, col, m);
            }
            let (icono, a_icono) = if sel { (col.map(|v| v + (255.0 - v) * 0.2), 1.0) } else { (BLANCO, 0.42) };
            lz.texto(&o.icono, x0 + 46.0, y + alto / 2.0, icono, a_icono * m);
            let base = if o.detalle.is_some() { 36.0 } else { 48.0 };
            lz.texto(&o.nombre, x0 + 92.0, y + base, [243.0, 239.0, 248.0], if sel { 1.0 } else { 0.62 } * m);
            if let Some(d) = &o.detalle {
                lz.texto(d, x0 + 92.0, y + 61.0, [243.0, 239.0, 248.0], if sel { 0.55 } else { 0.3 } * m);
            }
        }
        lz.texto(&r::MENU_PIE, W / 2.0, y0 + n * alto + 50.0, CLARO, 0.38 * m);
    }
}
